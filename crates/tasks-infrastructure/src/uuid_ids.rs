use tasks_application::ports::IdGenerator;
use tasks_domain::TaskId;
use uuid::Uuid;

/// Random (v4) UUIDs in hyphenated lowercase form.
#[derive(Debug, Default, Clone, Copy)]
pub struct UuidGenerator;

impl IdGenerator for UuidGenerator {
    fn next_id(&self) -> TaskId {
        TaskId::new(Uuid::new_v4().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_parseable_v4_uuids() {
        let id = UuidGenerator.next_id();
        let parsed = Uuid::parse_str(id.as_str()).unwrap();
        assert_eq!(parsed.get_version_num(), 4);
    }

    #[test]
    fn generates_distinct_ids() {
        assert_ne!(UuidGenerator.next_id(), UuidGenerator.next_id());
    }
}
