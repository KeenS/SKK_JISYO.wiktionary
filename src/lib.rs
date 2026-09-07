use quick_xml::events::Event;
use quick_xml::{Reader, XmlVersion};
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub mod model;
pub mod seikana;

/// Iterates `<page>` elements without deserializing the whole dump into memory.
pub fn articles(xml_file: impl AsRef<Path>) -> impl Iterator<Item = model::Page> {
    PageIterator::new(xml_file)
}

/// Iterates single-character kanji pages whose id is listed in `ids_file`.
pub fn kanji_articles(
    ids_file: impl AsRef<Path>,
    xml_file: impl AsRef<Path>,
) -> impl Iterator<Item = model::Page> {
    let ids = read_ids(ids_file);
    PageIterator::new(xml_file)
        .filter(move |page| ids.contains(&page.id) && page.title.chars().count() == 1)
}

struct PageIterator {
    reader: Reader<BufReader<File>>,
    buffer: Vec<u8>,
    state: PageState,
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
    fn new(path: impl AsRef<Path>) -> Self {
        let mut reader = Reader::from_file(path).expect("failed to read file");
        reader.config_mut().check_end_names = false;
        reader.config_mut().trim_text_start = false;
        Self {
            reader,
            buffer: Vec::new(),
            state: PageState::OutsidePage,
        }
    }
}

impl Iterator for PageIterator {
    type Item = model::Page;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let event = match self.reader.read_event_into(&mut self.buffer) {
                Ok(event) => event,
                Err(error) => {
                    eprintln!(
                        "xml_xtract: XML parsing stopped at offset {}: {error}",
                        self.reader.buffer_position()
                    );
                    self.state = PageState::Finished;
                    return None;
                }
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
                        .expect("invalid character reference")
                        .map(|ch| ch.to_string()),
                },
                _ => None,
            };
            match (event, &mut self.state) {
                (Event::Eof, _) => {
                    self.state = PageState::Finished;
                    return None;
                }
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
                        ..
                    } = std::mem::take(page);
                    self.state = PageState::OutsidePage;
                    let Some(title) = title else { continue };
                    let Some(id) = id else { continue };
                    let Some(revision) = revision else { continue };
                    return Some(model::Page {
                        ns: ns.unwrap_or(0),
                        id,
                        title,
                        revision,
                    });
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
                    let revision = model::Revision {
                        id: revision.id?,
                        comment: revision.comment.clone(),
                        text: revision.text.clone()?,
                    };
                    page.revision = Some(revision);
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
        Some(b"id") if page.id.is_none() => {
            page.id = Some(field.content.trim().parse().expect("page id"));
        }
        Some(b"title") if page.title.is_none() => page.title = Some(field.content),
        _ => {}
    }
}

fn assign_revision_field(revision: &mut RevisionData, field: FieldBuffer) {
    match field.name.as_deref() {
        Some(b"id") if revision.id.is_none() => {
            revision.id = Some(field.content.trim().parse().expect("revision id"));
        }
        Some(b"comment") if revision.comment.is_none() => {
            revision.comment = Some(field.content);
        }
        Some(b"text") if revision.text.is_none() => revision.text = Some(field.content),
        _ => {}
    }
}

fn read_ids(path: impl AsRef<Path>) -> HashSet<u64> {
    BufReader::new(File::open(path).expect("failed to open file"))
        .lines()
        .map(|line| {
            line.expect("line error")
                .trim()
                .parse::<u64>()
                .expect("parse error")
        })
        .collect()
}
