use super::Field;
use m_bus_oms_rules::telegram::ApplicationProtocolEvidence;

pub(super) struct Message {
    tag: u64,
    group: u64,
    addressed: Option<bool>,
}

impl Message {
    pub(super) fn new(tag: u64, group: u64, fields: &[Field<'_>]) -> Self {
        let server_position = match tag {
            0x0100 => Some(3),
            0x0300 | 0x0400 | 0x0500 | 0x0600 => Some(0),
            0x0700 | 0x0800 | 0x0900 | 0x0a00 => Some(1),
            _ => None,
        };
        let addressed = server_position.and_then(|pos| match fields.get(pos)? {
            Field::Bytes(id) => Some(!id.is_empty()),
            _ => None,
        });
        Self {
            tag,
            group,
            addressed,
        }
    }
}

fn groups(messages: &[Message]) -> bool {
    let mut visited = [false; 256];
    let mut previous = None;
    for message in messages {
        if previous == Some(message.group) {
            continue;
        }
        let Some(used) = usize::try_from(message.group)
            .ok()
            .and_then(|n| visited.get_mut(n))
        else {
            return false;
        };
        if *used {
            return false;
        }
        *used = true;
        previous = Some(message.group);
    }
    true
}

pub(super) fn check(mut messages: &[Message], evidence: &mut ApplicationProtocolEvidence) {
    evidence.sml_file = Some(false);
    let mut ordered_groups = true;
    if let [open, ..] = messages {
        if open.tag == 0x0100 {
            let Some(end) = messages.iter().position(|m| m.tag == 0x0200) else {
                return;
            };
            let Some((request, tail)) = messages.split_at_checked(end + 1) else {
                return;
            };
            if request
                .iter()
                .skip(1)
                .any(|m| m.tag == 0x0100 || m.tag & 1 != 0)
            {
                return;
            }
            ordered_groups &= groups(request);
            if let Some(address) = open.addressed {
                let mut peers = request
                    .iter()
                    .skip(1)
                    .filter_map(|message| message.addressed)
                    .peekable();
                if peers.peek().is_some() {
                    evidence.sml_file_address = Some(peers.all(|peer| peer == address));
                }
            }
            messages = tail;
        }
    }
    while let Some(first) = messages.first() {
        if !matches!(first.tag, 0x0101 | 0xff01) {
            return;
        }
        let mut end = None;
        let mut previous = first.tag;
        for (index, message) in messages.iter().enumerate().skip(1) {
            if message.tag == 0x0101 && previous == 0xff01 {
                end = Some(index);
                break;
            }
            if message.tag & 1 == 0 || message.tag == 0x0101 {
                return;
            }
            if message.tag == 0x0201 {
                end = Some(index + 1);
                break;
            }
            previous = message.tag;
        }
        let end = match end {
            Some(end) => end,
            None if messages.last().is_some_and(|m| m.tag == 0xff01) => messages.len(),
            None => return,
        };
        let Some((response, tail)) = messages.split_at_checked(end) else {
            return;
        };
        ordered_groups &= groups(response);
        messages = tail;
    }
    evidence.sml_file = Some(true);
    evidence.sml_file_groups = Some(ordered_groups);
}
