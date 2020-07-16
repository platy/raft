use super::Term;
use core::cmp::Ordering;

pub type Index = usize;

pub struct Item<Command> {
    pub term: Term,
    pub command: Command,
}

impl<Command> Item<Command> {
    pub fn new(term: Term, command: Command) -> Self {
        Self { term, command }
    }
}

pub trait Log {
    type Command;

    /// Check items against existing items in log, remove from log any conflicting items and those following them and append the remaining new items.
    /// # Returns
    /// Index of the last new item added
    /// #Panics
    /// If prev_index doesn't point at an existing log entry
    fn truncate_if_different_and_append(
        &mut self,
        prev_index: Index,
        items: impl IntoIterator<Item = Item<Self::Command>>,
    ) -> Index;

    /// Check whether the log entry's term at the specified index matches
    fn log_term_matches(&self, index: Index, term: Term) -> bool;

    /// Raft determines which of two logs is more up-to-date by comparing the index and term of the last entries in the
    /// logs. If the logs have last entries with different terms, then the log with the later term is more up-to-date.
    /// If the logs end with the same term, then whichever log is longer is more up-to-date.
    fn cmp(&self, other_last_log_index: Index, other_last_log_term: Term) -> Ordering {
        let own_last_log_term = self.last_log_term();
        let own_last_log_index = self.last_index();
        own_last_log_term
            .cmp(&other_last_log_term)
            .then(own_last_log_index.cmp(&other_last_log_index))
    }

    fn last_log_term(&self) -> Term;
    fn last_index(&self) -> Index;
}

impl<Command> Log for Vec<Item<Command>> {
    type Command = Command;

    fn truncate_if_different_and_append(
        &mut self,
        mut prev_index: Index,
        items: impl IntoIterator<Item = Item<Self::Command>>,
    ) -> Index {
        assert!(prev_index <= self.len());
        let mut new_items = items.into_iter().peekable();
        let mut existing_items = self.iter().skip(prev_index);
        // advance over existing matching items
        while let (Some(existing), Some(new)) = (existing_items.next(), new_items.peek()) {
            if existing.term == new.term {
                prev_index += 1;
                new_items.next();
            } else {
                break;
            }
        }
        // @todo don't truncate if everything matches, this might be a new leader checking a long chain of messages that are all correct
        self.truncate(prev_index);
        self.extend(new_items);
        self.len()
    }

    fn log_term_matches(&self, index: Index, term: Term) -> bool {
        if let Some(item) = self.get(index - 1) {
            return item.term == term;
        }
        false
    }

    fn last_log_term(&self) -> Term {
        self.last().map_or(0, |e| e.term)
    }

    fn last_index(&self) -> Index {
        self.len()
    }
}

#[cfg(test)]
mod test_vec {
    use super::{Item, Log};
    use core::cmp::Ordering;

    #[test]
    fn truncate_if_different_and_append_append_if_empty() {
        let mut log = vec![];
        log.truncate_if_different_and_append(0, vec![Item::new(2, 98)]);
        assert_eq!(log.len(), 1);
    }

    #[test]
    fn truncate_if_different_and_append_append_if_extending() {
        let mut log = vec![Item::new(1, 99)];
        log.truncate_if_different_and_append(1, vec![Item::new(2, 98)]);
        assert_eq!(log.len(), 2);
    }

    #[test]
    fn truncate_if_different_and_append_nothing_if_no_new_items() {
        let mut log = vec![Item::new(1, 99)];
        log.truncate_if_different_and_append(1, vec![]);
        assert_eq!(log.len(), 1);
    }

    #[test]
    fn truncate_if_different_and_append_replace_if_different() {
        let mut log = vec![Item::new(1, 99), Item::new(1, 99)];
        log.truncate_if_different_and_append(1, vec![Item::new(2, 98)]);
        assert_eq!(log.len(), 2);
        assert_eq!(log[1].term, 2);
    }

    #[test]
    fn truncate_if_different_and_append_truncate_if_different() {
        let mut log = vec![Item::new(1, 99), Item::new(1, 99), Item::new(1, 99)];
        log.truncate_if_different_and_append(1, vec![Item::new(2, 98)]);
        assert_eq!(log.len(), 2);
        assert_eq!(log[1].term, 2);
    }

    #[test]
    fn truncate_if_different_and_append_truncate_if_second_different() {
        let mut log = vec![
            Item::new(1, 99),
            Item::new(1, 99),
            Item::new(1, 99),
            Item::new(1, 99),
        ];
        log.truncate_if_different_and_append(1, vec![Item::new(1, 99), Item::new(2, 98)]);
        assert_eq!(log.len(), 3);
        assert_eq!(log[2].term, 2);
    }

    #[test]
    #[should_panic]
    fn truncate_if_different_and_append_panics_if_prev_too_high() {
        let mut log = vec![Item::new(1, 99)];
        log.truncate_if_different_and_append(3, vec![Item::new(1, 99), Item::new(2, 98)]);
    }

    #[test]
    fn log_term_matches_not_exist() {
        assert!(!vec![Item::new(4, 99)].log_term_matches(2, 5));
    }

    #[test]
    fn log_term_matches_wrong_term() {
        assert!(!vec![Item::new(4, 99)].log_term_matches(1, 5));
    }

    #[test]
    fn log_term_matches_matches() {
        assert!(vec![Item::new(4, 99)].log_term_matches(1, 4));
    }

    #[test]
    fn cmp_greater() {
        assert_eq!(
            Ordering::Greater,
            vec![Item::new(2, 99)].cmp(1, 1),
            "greater due to last term"
        );
        assert_eq!(
            Ordering::Greater,
            vec![Item::new(2, 99), Item::new(2, 99)].cmp(1, 2),
            "greater due to higher index"
        );
    }

    #[test]
    fn cmp_equal() {
        assert_eq!(
            Ordering::Equal,
            vec![Item::new(1, 99)].cmp(1, 1),
            "term and index equal"
        );
    }

    #[test]
    fn cmp_less() {
        assert_eq!(
            Ordering::Less,
            vec![Item::new(1, 99)].cmp(1, 2),
            "less due to last term"
        );
        assert_eq!(
            Ordering::Less,
            vec![Item::new(1, 99), Item::new(1, 99)].cmp(3, 1),
            "less due to last index"
        );
    }
}
