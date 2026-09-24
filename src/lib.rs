use quick_xml::events::Event;
use quick_xml::{Reader, XmlVersion};
use std::collections::HashSet;
use std::fmt;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;

pub mod candidate_order;
pub mod gsi;
pub mod jion;
pub mod model;

/// A syntax error from the article dump. The iterator yields this once and then ends.
#[derive(Debug)]
pub struct XmlError {
    pub offset: u64,
    pub message: String,
}

impl fmt::Display for XmlError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "XML parsing stopped at offset {}: {}",
            self.offset, self.message
        )
    }
}

impl std::error::Error for XmlError {}

impl From<XmlError> for io::Error {
    fn from(error: XmlError) -> Self {
        io::Error::other(error.to_string())
    }
}

/// Failure while opening a dump or the kanji id list.
#[derive(Debug)]
pub enum KanjiArticlesError {
    Io(io::Error),
    Ids(String),
}

impl fmt::Display for KanjiArticlesError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "{error}"),
            Self::Ids(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for KanjiArticlesError {}

impl From<KanjiArticlesError> for io::Error {
    fn from(error: KanjiArticlesError) -> Self {
        io::Error::other(error.to_string())
    }
}

/// Iterates `<page>` elements without deserializing the whole dump into memory.
pub fn articles(xml_file: impl AsRef<Path>) -> io::Result<PageIterator> {
    PageIterator::new(xml_file)
}

/// Iterates single-character kanji pages whose id is listed in `ids_file`.
pub fn kanji_articles(
    ids_file: impl AsRef<Path>,
    xml_file: impl AsRef<Path>,
) -> Result<KanjiPages, KanjiArticlesError> {
    let ids = read_ids(ids_file).map_err(KanjiArticlesError::Ids)?;
    let pages = PageIterator::new(xml_file).map_err(KanjiArticlesError::Io)?;
    Ok(KanjiPages { pages, ids })
}

pub struct KanjiPages {
    pages: PageIterator,
    ids: HashSet<u64>,
}

impl KanjiPages {
    pub fn skipped_pages(&self) -> usize {
        self.pages.skipped_pages()
    }
}

impl Iterator for KanjiPages {
    type Item = Result<model::Page, XmlError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.pages.next()? {
                Ok(page) if self.ids.contains(&page.id) && page.title.chars().count() == 1 => {
                    return Some(Ok(page));
                }
                Ok(_) => {}
                Err(error) => return Some(Err(error)),
            }
        }
    }
}

pub struct PageIterator {
    reader: Reader<BufReader<File>>,
    buffer: Vec<u8>,
    state: PageState,
    skipped_pages: usize,
    failed: bool,
}

#[derive(Debug, Default)]
enum PageState {
    #[default]
    OutsidePage,
    Page(PageData),
    Revision(PageData, RevisionData),
    Finished,
}

#[derive(Debug, Default)]
struct PageData {
    ns: Option<u64>,
    id: Option<u64>,
    title: Option<String>,
    revision: Option<model::Revision>,
    field: Option<FieldBuffer>,
    ignored_depth: usize,
    invalid: bool,
}

#[derive(Debug, Default)]
struct RevisionData {
    id: Option<u64>,
    comment: Option<String>,
    text: Option<String>,
    field: Option<FieldBuffer>,
    ignored_depth: usize,
}

#[derive(Debug, Default)]
struct FieldBuffer {
    name: Option<Vec<u8>>,
    content: String,
}

impl PageIterator {
    fn new(path: impl AsRef<Path>) -> io::Result<Self> {
        let mut reader =
            Reader::from_file(path).map_err(|error| io::Error::other(error.to_string()))?;
        reader.config_mut().check_end_names = false;
        reader.config_mut().trim_text_start = false;
        Ok(Self {
            reader,
            buffer: Vec::new(),
            state: PageState::OutsidePage,
            skipped_pages: 0,
            failed: false,
        })
    }

    pub fn skipped_pages(&self) -> usize {
        self.skipped_pages
    }

    fn fail(&mut self, message: impl Into<String>) -> Option<Result<model::Page, XmlError>> {
        if self.failed {
            return None;
        }
        self.failed = true;
        let offset = self.reader.buffer_position();
        self.state = PageState::Finished;
        Some(Err(XmlError {
            offset,
            message: message.into(),
        }))
    }
}

impl Iterator for PageIterator {
    type Item = Result<model::Page, XmlError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.failed {
            return None;
        }
        loop {
            let event = match self.reader.read_event_into(&mut self.buffer) {
                Ok(event) => event,
                Err(error) => return self.fail(error.to_string()),
            };
            let name = match &event {
                Event::Start(start) => Some(start.name().0.as_bytes().to_vec()),
                Event::Empty(start) => Some(start.name().0.as_bytes().to_vec()),
                Event::End(end) => Some(end.name().0.as_bytes().to_vec()),
                _ => None,
            };
            let content = match &event {
                Event::Text(text) => Some(text.xml_content(XmlVersion::Implicit1_0).into_owned()),
                Event::CData(text) => Some(text.xml_content(XmlVersion::Implicit1_0).into_owned()),
                Event::GeneralRef(reference) => match reference.as_ref() {
                    "lt" => Some('<'.to_string()),
                    "gt" => Some('>'.to_string()),
                    "amp" => Some('&'.to_string()),
                    "apos" => Some('\''.to_string()),
                    "quot" => Some('"'.to_string()),
                    _ => reference
                        .resolve_char_ref()
                        .ok()
                        .flatten()
                        .map(|ch| ch.to_string()),
                },
                _ => None,
            };
            if matches!(event, Event::Eof) {
                let unfinished =
                    !matches!(self.state, PageState::OutsidePage | PageState::Finished);
                if unfinished {
                    return self.fail("unexpected end of file");
                }
                self.state = PageState::Finished;
                return None;
            }
            match (event, &mut self.state) {
                (Event::Start(_), PageState::OutsidePage) if name == Some(b"page".to_vec()) => {
                    self.state = PageState::Page(PageData::default());
                }
                (Event::Empty(_), PageState::OutsidePage) if name == Some(b"page".to_vec()) => {}
                (Event::Start(_), PageState::Page(page)) if name == Some(b"revision".to_vec()) => {
                    let page = std::mem::take(page);
                    self.state = PageState::Revision(page, RevisionData::default());
                }
                (Event::Start(_), PageState::Page(page)) => {
                    if page.ignored_depth > 0 || page.field.is_some() {
                        page.ignored_depth += 1;
                    } else {
                        page.field = Some(FieldBuffer {
                            name,
                            content: content.unwrap_or_default(),
                        });
                    }
                }
                (Event::Empty(_), PageState::Revision(_, revision))
                    if name == Some(b"text".to_vec()) =>
                {
                    revision.text.get_or_insert_with(String::new);
                }
                (Event::Start(_), PageState::Revision(_, revision)) => {
                    if revision.ignored_depth > 0 || revision.field.is_some() {
                        revision.ignored_depth += 1;
                    } else {
                        revision.field = Some(FieldBuffer {
                            name,
                            content: content.unwrap_or_default(),
                        });
                    }
                }
                (Event::Text(_), PageState::Page(page)) => {
                    if let Some(field) = &mut page.field {
                        field.content.push_str(&content.unwrap_or_default());
                    }
                }
                (Event::GeneralRef(_), PageState::Page(page)) => {
                    if let Some(field) = &mut page.field {
                        field.content.push_str(&content.unwrap_or_default());
                    }
                }
                (Event::Text(_), PageState::Revision(_, revision)) => {
                    if let Some(field) = &mut revision.field {
                        field.content.push_str(&content.unwrap_or_default());
                    }
                }
                (Event::GeneralRef(_), PageState::Revision(_, revision)) => {
                    if let Some(field) = &mut revision.field {
                        field.content.push_str(&content.unwrap_or_default());
                    }
                }
                (Event::CData(_), PageState::Page(page)) => {
                    if let Some(field) = &mut page.field {
                        field.content.push_str(&content.unwrap_or_default());
                    }
                }
                (Event::CData(_), PageState::Revision(_, revision)) => {
                    if let Some(field) = &mut revision.field {
                        field.content.push_str(&content.unwrap_or_default());
                    }
                }
                (Event::End(_), PageState::Page(page)) if name == Some(b"page".to_vec()) => {
                    let PageData {
                        ns,
                        id,
                        title,
                        revision,
                        invalid,
                        ..
                    } = std::mem::take(page);
                    self.state = PageState::OutsidePage;
                    let Some(title) = title else {
                        self.skipped_pages += 1;
                        continue;
                    };
                    let Some(id) = id else {
                        self.skipped_pages += 1;
                        continue;
                    };
                    let Some(revision) = revision else {
                        self.skipped_pages += 1;
                        continue;
                    };
                    if invalid {
                        self.skipped_pages += 1;
                        continue;
                    }
                    return Some(Ok(model::Page {
                        ns: ns.unwrap_or(0),
                        id,
                        title,
                        revision,
                    }));
                }
                (Event::End(_), PageState::Page(page)) => {
                    if page.ignored_depth > 0 {
                        page.ignored_depth -= 1;
                    } else if let Some(field) = page.field.take() {
                        assign_page_field(page, field);
                    }
                }
                (Event::End(_), PageState::Revision(page, revision))
                    if name == Some(b"revision".to_vec()) =>
                {
                    if let (Some(id), Some(text)) = (revision.id, revision.text.clone()) {
                        page.revision = Some(model::Revision {
                            id,
                            comment: revision.comment.clone(),
                            text,
                        });
                    }
                    let page = std::mem::take(page);
                    self.state = PageState::Page(page);
                }
                (Event::End(_), PageState::Revision(_, revision)) => {
                    if revision.ignored_depth > 0 {
                        revision.ignored_depth -= 1;
                    } else if let Some(field) = revision.field.take() {
                        assign_revision_field(revision, field);
                    }
                }
                _ => {}
            }
            self.buffer.clear();
        }
    }
}

fn assign_page_field(page: &mut PageData, field: FieldBuffer) {
    match field.name.as_deref() {
        Some(b"ns") if page.ns.is_none() => {
            page.ns = field.content.trim().parse().ok();
        }
        Some(b"id") if page.id.is_none() => match field.content.trim().parse() {
            Ok(id) => page.id = Some(id),
            Err(_) => page.invalid = true,
        },
        Some(b"title") if page.title.is_none() => page.title = Some(field.content),
        _ => {}
    }
}

fn assign_revision_field(revision: &mut RevisionData, field: FieldBuffer) {
    match field.name.as_deref() {
        Some(b"id") if revision.id.is_none() => {
            revision.id = field.content.trim().parse().ok();
        }
        Some(b"comment") if revision.comment.is_none() => {
            revision.comment = Some(field.content);
        }
        Some(b"text") if revision.text.is_none() => revision.text = Some(field.content),
        _ => {}
    }
}

fn read_ids(path: impl AsRef<Path>) -> Result<HashSet<u64>, String> {
    let file = File::open(&path)
        .map_err(|error| format!("failed to open {}: {error}", path.as_ref().display()))?;
    BufReader::new(file)
        .lines()
        .enumerate()
        .map(|(index, line)| {
            let line = line
                .map_err(|error| format!("failed to read {}: {error}", path.as_ref().display()))?;
            line.trim().parse::<u64>().map_err(|error| {
                format!(
                    "invalid page id in {} line {}: {error}",
                    path.as_ref().display(),
                    index + 1
                )
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn collect(xml: &str) -> (Vec<Result<model::Page, XmlError>>, usize) {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        xml.hash(&mut hasher);
        let path = std::env::temp_dir().join(format!("xml-xtract-parser-{}.xml", hasher.finish()));
        let mut file = File::create(&path).unwrap();
        file.write_all(xml.as_bytes()).unwrap();
        let mut pages = articles(&path).unwrap();
        let collected: Vec<_> = pages.by_ref().collect();
        let skipped = pages.skipped_pages();
        std::fs::remove_file(path).unwrap();
        (collected, skipped)
    }

    #[test]
    fn continues_after_a_revision_without_text() {
        let (pages, skipped) = collect(
            r#"<mediawiki>
<page><title>Skip</title><ns>0</ns><id>1</id><revision><id>2</id></revision></page>
<page><title>Keep</title><ns>0</ns><id>3</id><revision><id>4</id><text>body</text></revision></page>
</mediawiki>"#,
        );
        let pages = pages.into_iter().collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(skipped, 1);
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].title, "Keep");
        assert_eq!(pages[0].revision.text, "body");
    }

    #[test]
    fn stops_on_a_truncated_page_without_yielding_it() {
        let (pages, _) = collect(
            r#"<mediawiki>
<page><title>Good</title><ns>0</ns><id>1</id><revision><id>2</id><text>one</text></revision></page>
<page><title>Bad</title><ns>0</ns><id>3</id><revision><id>4</id><text>two</text></revision>
"#,
        );
        assert!(pages.iter().any(|page| page.is_err()));
        let titles: Vec<_> = pages
            .into_iter()
            .filter_map(Result::ok)
            .map(|page| page.title)
            .collect();
        assert_eq!(titles, vec!["Good".to_string()]);
    }

    #[test]
    fn ignores_an_unresolved_character_reference() {
        let (pages, _) = collect(
            r#"<mediawiki>
<page><title>字</title><ns>0</ns><id>1</id><revision><id>2</id><text>前&#x110000;後</text></revision></page>
</mediawiki>"#,
        );
        let pages = pages.into_iter().collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(pages.len(), 1);
        assert!(!pages[0].revision.text.contains('\u{fffd}'));
    }
}
