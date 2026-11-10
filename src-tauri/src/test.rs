
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant};

// ============================================================
// NOTES APP STRESS TESTING ENGINE
// ============================================================

#[derive(Clone, Debug, PartialEq)]
struct Note {
    id: u64,
    title: String,
    content: String,
    tags: Vec<String>,
    created_at: u64,
    updated_at: u64,
    is_favorite: bool,
    is_archived: bool,
    is_deleted: bool,
    word_count: usize,
}

impl Note {
    fn new(id: u64, title: &str, content: &str) -> Self {
        Self {
            id,
            title: title.to_string(),
            content: content.to_string(),
            tags: Vec::new(),
            created_at: id,
            updated_at: id,
            is_favorite: false,
            is_archived: false,
            is_deleted: false,
            word_count: content.split_whitespace().count(),
        }
    }

    fn update_content(&mut self, content: &str, timestamp: u64) {
        self.content = content.to_string();
        self.updated_at = timestamp;
        self.word_count = content.split_whitespace().count();
    }

    fn add_tag(&mut self, tag: &str) {
        if !self.tags.contains(&tag.to_string()) {
            self.tags.push(tag.to_string());
        }
    }

    fn remove_tag(&mut self, tag: &str) {
        self.tags.retain(|t| t != tag);
    }

    fn contains(&self, query: &str) -> bool {
        let query = query.to_lowercase();

        self.title.to_lowercase().contains(&query)
            || self.content.to_lowercase().contains(&query)
            || self.tags.iter().any(|t| {
                t.to_lowercase().contains(&query)
            })
    }

    fn validate(&self) -> bool {
        self.word_count == self.content.split_whitespace().count()
    }
}

// ============================================================
// DATABASE
// ============================================================

#[derive(Clone)]
struct Database {
    notes: HashMap<u64, Note>,
    next_id: u64,
}

impl Database {
    fn new() -> Self {
        Self {
            notes: HashMap::new(),
            next_id: 1,
        }
    }

    fn create(&mut self, title: &str, content: &str) -> u64 {
        let id = self.next_id;
        self.next_id += 1;

        let note = Note::new(id, title, content);
        self.notes.insert(id, note);

        id
    }

    fn get(&self, id: u64) -> Option<&Note> {
        self.notes.get(&id)
    }

    fn update(
        &mut self,
        id: u64,
        title: &str,
        content: &str,
    ) -> bool {
        if let Some(note) = self.notes.get_mut(&id) {
            note.title = title.to_string();
            note.update_content(content, id + 1000);
            true
        } else {
            false
        }
    }

    fn delete(&mut self, id: u64) -> bool {
        if let Some(note) = self.notes.get_mut(&id) {
            note.is_deleted = true;
            true
        } else {
            false
        }
    }

    fn restore(&mut self, id: u64) -> bool {
        if let Some(note) = self.notes.get_mut(&id) {
            note.is_deleted = false;
            true
        } else {
            false
        }
    }

    fn permanent_delete(&mut self, id: u64) -> bool {
        self.notes.remove(&id).is_some()
    }

    fn count(&self) -> usize {
        self.notes.len()
    }

    fn active_notes(&self) -> Vec<&Note> {
        self.notes.values()
            .filter(|n| !n.is_deleted && !n.is_archived)
            .collect()
    }

    fn archived_notes(&self) -> Vec<&Note> {
        self.notes.values()
            .filter(|n| n.is_archived && !n.is_deleted)
            .collect()
    }

    fn favorite_notes(&self) -> Vec<&Note> {
        self.notes.values()
            .filter(|n| n.is_favorite && !n.is_deleted)
            .collect()
    }
}

// ============================================================
// TAG MANAGEMENT
// ============================================================

struct TagManager;

impl TagManager {
    fn filter_by_tag<'a>(
        notes: &'a Database,
        tag: &str,
    ) -> Vec<&'a Note> {
        notes.notes.values()
            .filter(|n| {
                !n.is_deleted
                    && n.tags.iter().any(|t| t == tag)
            })
            .collect()
    }

    fn all_tags(db: &Database) -> HashSet<String> {
        let mut tags = HashSet::new();

        for note in db.notes.values() {
            for tag in &note.tags {
                tags.insert(tag.clone());
            }
        }

        tags
    }

    fn most_used_tag(db: &Database) -> Option<String> {
        let mut counts: HashMap<String, usize> = HashMap::new();

        for note in db.notes.values() {
            for tag in &note.tags {
                *counts.entry(tag.clone()).or_insert(0) += 1;
            }
        }

        counts.into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(tag, _)| tag)
    }
}

// ============================================================
// SEARCH ALGORITHMS
// ============================================================

struct SearchEngine;

impl SearchEngine {
    fn linear_search<'a>(
        notes: &'a Database,
        query: &str,
    ) -> Vec<&'a Note> {
        notes.notes.values()
            .filter(|n| !n.is_deleted && n.contains(query))
            .collect()
    }

    fn binary_search<'a>(
        notes: &'a Database,
        id: u64,
    ) -> Option<&'a Note> {
        // Binary search over sorted note IDs.
        let mut ids: Vec<u64> = notes.notes.keys()
            .copied()
            .collect();

        ids.sort_unstable();

        let mut left = 0;
        let mut right = ids.len();

        while left < right {
            let mid = left + (right - left) / 2;

            if ids[mid] == id {
                return notes.get(id);
            }

            if ids[mid] < id {
                left = mid + 1;
            } else {
                right = mid;
            }
        }

        None
    }

    fn prefix_search<'a>(
        notes: &'a Database,
        prefix: &str,
    ) -> Vec<&'a Note> {
        let prefix = prefix.to_lowercase();

        notes.notes.values()
            .filter(|n| {
                !n.is_deleted
                    && n.title.to_lowercase().starts_with(&prefix)
            })
            .collect()
    }
}

// ============================================================
// SORTING ALGORITHMS
// ============================================================

struct SortEngine;

impl SortEngine {
    fn by_created<'a>(mut notes: Vec<&'a Note>) -> Vec<&'a Note> {
        notes.sort_by_key(|n| n.created_at);
        notes
    }

    fn by_updated<'a>(mut notes: Vec<&'a Note>) -> Vec<&'a Note> {
        notes.sort_by_key(|n| n.updated_at);
        notes.reverse();
        notes
    }

    fn by_title<'a>(mut notes: Vec<&'a Note>) -> Vec<&'a Note> {
        notes.sort_by(|a, b| {
            a.title.to_lowercase().cmp(&b.title.to_lowercase())
        });
        notes
    }

    fn by_word_count<'a>(mut notes: Vec<&'a Note>) -> Vec<&'a Note> {
        notes.sort_by_key(|n| n.word_count);
        notes.reverse();
        notes
    }

    fn merge_sort<T: Ord + Clone>(arr: &mut [T]) {
        if arr.len() <= 1 {
            return;
        }

        let mid = arr.len() / 2;

        Self::merge_sort(&mut arr[..mid]);
        Self::merge_sort(&mut arr[mid..]);

        let left = arr[..mid].to_vec();
        let right = arr[mid..].to_vec();

        let mut i = 0;
        let mut j = 0;
        let mut k = 0;

        while i < left.len() && j < right.len() {
            if left[i] <= right[j] {
                arr[k] = left[i].clone();
                i += 1;
            } else {
                arr[k] = right[j].clone();
                j += 1;
            }

            k += 1;
        }

        while i < left.len() {
            arr[k] = left[i].clone();
            i += 1;
            k += 1;
        }

        while j < right.len() {
            arr[k] = right[j].clone();
            j += 1;
            k += 1;
        }
    }

    fn quick_sort(arr: &mut [i64]) {
        if arr.len() <= 1 {
            return;
        }

        let pivot = arr[arr.len() / 2];

        let mut left = 0;
        let mut right = arr.len() - 1;

        loop {
            while arr[left] < pivot {
                left += 1;
            }

            while arr[right] > pivot {
                if right == 0 {
                    break;
                }
                right -= 1;
            }

            if left >= right {
                break;
            }

            arr.swap(left, right);
            left += 1;

            if right == 0 {
                break;
            }
            right -= 1;
        }

        let split = left;

        if split > 0 {
            Self::quick_sort(&mut arr[..split]);
        }

        if split < arr.len() {
            Self::quick_sort(&mut arr[split..]);
        }
    }
}

// ============================================================
// DUPLICATE DETECTION
// ============================================================

struct DuplicateDetector;

impl DuplicateDetector {
    fn detect_duplicates(db: &Database) -> Vec<Vec<u64>> {
        let mut content_map: HashMap<String, Vec<u64>> =
            HashMap::new();

        for note in db.notes.values() {
            if !note.is_deleted {
                let normalized = note.content
                    .trim()
                    .to_lowercase();

                content_map.entry(normalized)
                    .or_default()
                    .push(note.id);
            }
        }

        content_map.into_values()
            .filter(|ids| ids.len() > 1)
            .collect()
    }

    fn has_duplicate(db: &Database, content: &str) -> bool {
        let normalized = content.trim().to_lowercase();

        db.notes.values().any(|n| {
            !n.is_deleted
                && n.content.trim().to_lowercase() == normalized
        })
    }
}

// ============================================================
// UNDO / REDO
// ============================================================

struct History {
    undo_stack: Vec<Database>,
    redo_stack: Vec<Database>,
    max_history: usize,
}

impl History {
    fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_history: 50,
        }
    }

    fn save(&mut self, db: &Database) {
        self.undo_stack.push(db.clone());

        if self.undo_stack.len() > self.max_history {
            self.undo_stack.remove(0);
        }

        self.redo_stack.clear();
    }

    fn undo(&mut self, db: &mut Database) -> bool {
        if let Some(previous) = self.undo_stack.pop() {
            self.redo_stack.push(db.clone());
            *db = previous;
            true
        } else {
            false
        }
    }

    fn redo(&mut self, db: &mut Database) -> bool {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(db.clone());
            *db = next;
            true
        } else {
            false
        }
    }
}

// ============================================================
// PAGINATION
// ============================================================

struct Pagination;

impl Pagination {
    fn get_page<'a>(
        notes: &[&'a Note],
        page: usize,
        page_size: usize,
    ) -> Vec<&'a Note> {
        if page_size == 0 {
            return Vec::new();
        }

        let start = match page.checked_mul(page_size) {
            Some(value) => value,
            None => return Vec::new(),
        };

        notes.iter()
            .skip(start)
            .take(page_size)
            .copied()
            .collect()
    }

    fn total_pages(total: usize, page_size: usize) -> usize {
        if page_size == 0 {
            return 0;
        }

        total.div_ceil(page_size)
    }
}

// ============================================================
// NOTE ARCHIVE AND FAVORITES
// ============================================================

struct NoteActions;

impl NoteActions {
    fn toggle_favorite(db: &mut Database, id: u64) -> bool {
        if let Some(note) = db.notes.get_mut(&id) {
            note.is_favorite = !note.is_favorite;
            return true;
        }

        false
    }

    fn archive(db: &mut Database, id: u64) -> bool {
        if let Some(note) = db.notes.get_mut(&id) {
            note.is_archived = true;
            return true;
        }

        false
    }

    fn unarchive(db: &mut Database, id: u64) -> bool {
        if let Some(note) = db.notes.get_mut(&id) {
            note.is_archived = false;
            return true;
        }

        false
    }
}

// ============================================================
// DATA VALIDATION
// ============================================================

struct Validator;

impl Validator {
    fn validate_database(db: &Database) -> Vec<String> {
        let mut errors = Vec::new();

        for (id, note) in &db.notes {
            if *id != note.id {
                errors.push(format!(
                    "ID mismatch: map={}, note={}",
                    id, note.id
                ));
            }

            if note.title.trim().is_empty() {
                errors.push(format!(
                    "Empty title in note {}",
                    id
                ));
            }

            if !note.validate() {
                errors.push(format!(
                    "Word count mismatch in note {}",
                    id
                ));
            }

            let mut seen = HashSet::new();

            for tag in &note.tags {
                if !seen.insert(tag) {
                    errors.push(format!(
                        "Duplicate tag in note {}",
                        id
                    ));
                }
            }
        }

        errors
    }

    fn validate_unique_ids(db: &Database) -> bool {
        let ids: HashSet<u64> = db.notes.keys().copied().collect();
        ids.len() == db.notes.len()
    }
}

// ============================================================
// TEXT ANALYTICS
// ============================================================

struct TextAnalytics;

impl TextAnalytics {
    fn character_count(note: &Note) -> usize {
        note.content.chars().count()
    }

    fn average_word_length(note: &Note) -> f64 {
        let words: Vec<&str> =
            note.content.split_whitespace().collect();

        if words.is_empty() {
            return 0.0;
        }

        let total: usize = words.iter()
            .map(|w| w.chars().count())
            .sum();

        total as f64 / words.len() as f64
    }

    fn most_common_word(note: &Note) -> Option<String> {
        let mut counts: HashMap<String, usize> =
            HashMap::new();

        for word in note.content.split_whitespace() {
            let normalized = word
                .to_lowercase()
                .trim_matches(|c: char| !c.is_alphanumeric())
                .to_string();

            if !normalized.is_empty() {
                *counts.entry(normalized).or_insert(0) += 1;
            }
        }

        counts.into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(word, _)| word)
    }

    fn reading_time(note: &Note) -> usize {
        // Assumption: 200 words per minute.
        note.word_count.div_ceil(200).max(1)
    }
}

// ============================================================
// RANDOM TEST DATA GENERATOR
// ============================================================

struct TestDataGenerator {
    seed: u64,
}

impl TestDataGenerator {
    fn new(seed: u64) -> Self {
        Self { seed }
    }

    fn random(&mut self) -> u64 {
        self.seed = self.seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);

        self.seed
    }

    fn random_range(&mut self, min: u64, max: u64) -> u64 {
        if max <= min {
            return min;
        }

        min + self.random() % (max - min + 1)
    }

    fn random_title(&mut self) -> String {
        let subjects = [
            "Rust Programming",
            "Algorithms",
            "Database Design",
            "Project Ideas",
            "Machine Learning",
            "Web Development",
            "Personal Journal",
            "Important Reminder",
            "Meeting Notes",
            "Research Paper",
        ];

        let index = self.random_range(
            0,
            (subjects.len() - 1) as u64
        ) as usize;

        format!(
            "{} #{}",
            subjects[index],
            self.random_range(1, 9999)
        )
    }

    fn random_content(&mut self) -> String {
        let words = [
            "algorithm",
            "database",
            "function",
            "variable",
            "performance",
            "testing",
            "application",
            "system",
            "development",
            "programming",
            "implementation",
            "optimization",
        ];

        let count = self.random_range(5, 50);
        let mut result = String::new();

        for _ in 0..count {
            let index = self.random_range(
                0,
                (words.len() - 1) as u64
            ) as usize;

            result.push_str(words[index]);
            result.push(' ');
        }

        result
    }

    fn random_tag(&mut self) -> String {
        let tags = [
            "important",
            "work",
            "personal",
            "college",
            "coding",
            "ideas",
            "research",
            "projects",
            "urgent",
            "learning",
        ];

        let index = self.random_range(
            0,
            (tags.len() - 1) as u64
        ) as usize;

        tags[index].to_string()
    }
}

// ============================================================
// PERFORMANCE BENCHMARK
// ============================================================

struct Benchmark;

impl Benchmark {
    fn measure<F: FnOnce()>(
        name: &str,
        operation: F,
    ) -> Duration {
        let start = Instant::now();

        operation();

        let elapsed = start.elapsed();

        println!(
            "[BENCHMARK] {:30} {:?}",
            name,
            elapsed
        );

        elapsed
    }
}

// ============================================================
// TEST REPORT
// ============================================================

struct TestReport {
    passed: usize,
    failed: usize,
}

impl TestReport {
    fn new() -> Self {
        Self {
            passed: 0,
            failed: 0,
        }
    }

    fn assert(&mut self, condition: bool, name: &str) {
        if condition {
            self.passed += 1;
            println!("[PASS] {}", name);
        } else {
            self.failed += 1;
            println!("[FAIL] {}", name);
        }
    }

    fn print(&self) {
        println!("\n================================");
        println!("         TEST SUMMARY");
        println!("================================");
        println!("Passed: {}", self.passed);
        println!("Failed: {}", self.failed);
        println!(
            "Success rate: {:.2}%",
            if self.passed + self.failed == 0 {
                0.0
            } else {
                self.passed as f64
                    / (self.passed + self.failed) as f64
                    * 100.0
            }
        );
        println!("================================");
    }
}

// ============================================================
// MAIN TEST SUITE
// ============================================================

fn main() {
    println!("NOTES APP STRESS TESTER");
    println!("=======================\n");

    let mut db = Database::new();
    let mut history = History::new();
    let mut report = TestReport::new();

    // --------------------------------------------------------
    // TEST 1: CREATE NOTES
    // --------------------------------------------------------

    println!("\nTEST 1: CREATE NOTES");

    let id1 = db.create(
        "Rust Learning",
        "Learning ownership and borrowing"
    );

    let id2 = db.create(
        "Project Ideas",
        "Build a modern notes application"
    );

    let id3 = db.create(
        "DSA Preparation",
        "Study sorting searching and graphs"
    );

    report.assert(db.count() == 3, "Create three notes");
    report.assert(
        db.get(id1).is_some(),
        "Retrieve created note"
    );

    // --------------------------------------------------------
    // TEST 2: UPDATE NOTES
    // --------------------------------------------------------

    println!("\nTEST 2: UPDATE NOTES");

    history.save(&db);

    let updated = db.update(
        id1,
        "Advanced Rust",
        "Ownership borrowing lifetimes traits"
    );

    report.assert(updated, "Update existing note");

    report.assert(
        db.get(id1).unwrap().title == "Advanced Rust",
        "Updated title matches"
    );

    report.assert(
        !db.update(9999, "Missing", "Missing note"),
        "Reject invalid note ID"
    );

    // --------------------------------------------------------
    // TEST 3: TAGS
    // --------------------------------------------------------

    println!("\nTEST 3: TAG MANAGEMENT");

    db.notes.get_mut(&id1).unwrap()
        .add_tag("coding");

    db.notes.get_mut(&id1).unwrap()
        .add_tag("college");

    db.notes.get_mut(&id2).unwrap()
        .add_tag("projects");

    report.assert(
        db.notes.get(&id1).unwrap().tags.len() == 2,
        "Add tags"
    );

    db.notes.get_mut(&id1).unwrap()
        .remove_tag("college");

    report.assert(
        db.notes.get(&id1).unwrap().tags.len() == 1,
        "Remove tag"
    );

    report.assert(
        TagManager::filter_by_tag(&db, "coding").len() == 1,
        "Filter notes by tag"
    );

    // --------------------------------------------------------
    // TEST 4: SEARCH
    // --------------------------------------------------------

    println!("\nTEST 4: SEARCH");

    let search_results =
        SearchEngine::linear_search(&db, "Rust");

    report.assert(
        !search_results.is_empty(),
        "Search by content"
    );

    let prefix_results =
        SearchEngine::prefix_search(&db, "Advanced");

    report.assert(
        !prefix_results.is_empty(),
        "Search by title prefix"
    );

    report.assert(
        SearchEngine::binary_search(&db, id2).is_some(),
        "Binary search by ID"
    );

    // --------------------------------------------------------
    // TEST 5: SORTING
    // --------------------------------------------------------

    println!("\nTEST 5: SORTING");

    let notes = db.active_notes();

    let sorted = SortEngine::by_title(notes);

    report.assert(
        sorted.windows(2).all(|w| {
            w[0].title.to_lowercase()
                <= w[1].title.to_lowercase()
        }),
        "Sort notes alphabetically"
    );

    let mut numbers = vec![9, 2, 7, 1, 5, 3];

    SortEngine::quick_sort(&mut numbers);

    report.assert(
        numbers == vec![1, 2, 3, 5, 7, 9],
        "Quick sort correctness"
    );

    let mut merge_numbers = vec![8, 3, 6, 1, 9, 2];

    SortEngine::merge_sort(&mut merge_numbers);

    report.assert(
        merge_numbers == vec![1, 2, 3, 6, 8, 9],
        "Merge sort correctness"
    );

    // --------------------------------------------------------
    // TEST 6: FAVORITES
    // --------------------------------------------------------

    println!("\nTEST 6: FAVORITES");

    NoteActions::toggle_favorite(&mut db, id1);

    report.assert(
        db.notes.get(&id1).unwrap().is_favorite,
        "Mark note as favorite"
    );

    NoteActions::toggle_favorite(&mut db, id1);

    report.assert(
        !db.notes.get(&id1).unwrap().is_favorite,
        "Unmark favorite"
    );

    // --------------------------------------------------------
    // TEST 7: ARCHIVE
    // --------------------------------------------------------

    println!("\nTEST 7: ARCHIVE");

    NoteActions::archive(&mut db, id2);

    report.assert(
        db.archived_notes().len() == 1,
        "Archive note"
    );

    NoteActions::unarchive(&mut db, id2);

    report.assert(
        db.archived_notes().is_empty(),
        "Unarchive note"
    );

    // --------------------------------------------------------
    // TEST 8: DELETE AND RESTORE
    // --------------------------------------------------------

    println!("\nTEST 8: DELETE AND RESTORE");

    db.delete(id3);

    report.assert(
        db.notes.get(&id3).unwrap().is_deleted,
        "Soft delete note"
    );

    report.assert(
        db.active_notes().len() == 2,
        "Exclude deleted notes"
    );

    db.restore(id3);

    report.assert(
        !db.notes.get(&id3).unwrap().is_deleted,
        "Restore deleted note"
    );

    // --------------------------------------------------------
    // TEST 9: DUPLICATE DETECTION
    // --------------------------------------------------------

    println!("\nTEST 9: DUPLICATE DETECTION");

    db.create("Duplicate A", "same content");
    db.create("Duplicate B", "SAME CONTENT");

    let duplicates =
        DuplicateDetector::detect_duplicates(&db);

    report.assert(
        !duplicates.is_empty(),
        "Detect duplicate content"
    );

    report.assert(
        DuplicateDetector::has_duplicate(&db, "same content"),
        "Check duplicate content"
    );

    // --------------------------------------------------------
    // TEST 10: UNDO AND REDO
    // --------------------------------------------------------

    println!("\nTEST 10: UNDO / REDO");

    let previous_count = db.count();

    history.save(&db);

    db.create("Temporary Note", "This will be undone");

    report.assert(
        db.count() == previous_count + 1,
        "Create note before undo"
    );

    report.assert(
        history.undo(&mut db),
        "Undo operation"
    );

    report.assert(
        db.count() == previous_count,
        "Undo restores previous database"
    );

    report.assert(
        history.redo(&mut db),
        "Redo operation"
    );

    report.assert(
        db.count() == previous_count + 1,
        "Redo restores latest database"
    );

    // --------------------------------------------------------
    // TEST 11: PAGINATION
    // --------------------------------------------------------

    println!("\nTEST 11: PAGINATION");

    let notes = db.active_notes();

    let page = Pagination::get_page(&notes, 0, 2);

    report.assert(
        page.len() == 2,
        "First page contains two notes"
    );

    report.assert(
        Pagination::total_pages(10, 3) == 4,
        "Calculate total pages"
    );

    report.assert(
        Pagination::get_page(&notes, 0, 0).is_empty(),
        "Reject zero page size"
    );

    // --------------------------------------------------------
    // TEST 12: TEXT ANALYTICS
    // --------------------------------------------------------

    println!("\nTEST 12: TEXT ANALYTICS");

    let note = db.get(id1).unwrap();

    report.assert(
        TextAnalytics::character_count(note) > 0,
        "Calculate character count"
    );

    report.assert(
        TextAnalytics::average_word_length(note) > 0.0,
        "Calculate average word length"
    );

    report.assert(
        TextAnalytics::reading_time(note) >= 1,
        "Estimate reading time"
    );

    // --------------------------------------------------------
    // TEST 13: DATABASE VALIDATION
    // --------------------------------------------------------

    println!("\nTEST 13: DATA VALIDATION");

    let errors = Validator::validate_database(&db);

    report.assert(
        errors.is_empty(),
        "Database integrity"
    );

    report.assert(
        Validator::validate_unique_ids(&db),
        "Unique note IDs"
    );

    // --------------------------------------------------------
    // TEST 14: RANDOMIZED STRESS TEST
    // --------------------------------------------------------

    println!("\nTEST 14: RANDOMIZED STRESS TEST");

    let mut generator = TestDataGenerator::new(12345);

    for i in 0..1000 {
        let operation = generator.random_range(0, 5);

        match operation {
            0 => {
                let title = generator.random_title();
                let content = generator.random_content();

                db.create(&title, &content);
            }

            1 => {
                let id = generator.random_range(
                    1,
                    db.next_id.saturating_sub(1)
                );

                if db.notes.contains_key(&id) {
                    let title = generator.random_title();
                    let content = generator.random_content();

                    db.update(id, &title, &content);
                }
            }

            2 => {
                let id = generator.random_range(
                    1,
                    db.next_id.saturating_sub(1)
                );

                if let Some(note) = db.notes.get_mut(&id) {
                    note.add_tag(&generator.random_tag());
                }
            }

            3 => {
                let id = generator.random_range(
                    1,
                    db.next_id.saturating_sub(1)
                );

                NoteActions::toggle_favorite(&mut db, id);
            }

            4 => {
                let id = generator.random_range(
                    1,
                    db.next_id.saturating_sub(1)
                );

                if db.notes.contains_key(&id) {
                    db.delete(id);
                }
            }

            _ => unreachable!(),
        }

        if i % 100 == 0 {
            println!("Stress test progress: {} / 1000", i);
        }
    }

    let errors = Validator::validate_database(&db);

    report.assert(
        errors.is_empty(),
        "Randomized stress test integrity"
    );

    println!("Total notes generated: {}", db.count());

    // --------------------------------------------------------
    // TEST 15: PERFORMANCE
    // --------------------------------------------------------

    println!("\nTEST 15: PERFORMANCE BENCHMARKS");

    Benchmark::measure("Linear search", || {
        for _ in 0..1000 {
            SearchEngine::linear_search(&db, "algorithm");
        }
    });

    Benchmark::measure("Binary search", || {
        for _ in 0..1000 {
            SearchEngine::binary_search(&db, 10);
        }
    });

    Benchmark::measure("Tag extraction", || {
        for _ in 0..1000 {
            TagManager::all_tags(&db);
        }
    });

    Benchmark::measure("Duplicate detection", || {
        DuplicateDetector::detect_duplicates(&db);
    });

    Benchmark::measure("Database validation", || {
        Validator::validate_database(&db);
    });

    // --------------------------------------------------------
    // FINAL REPORT
    // --------------------------------------------------------

    report.print();

    println!("\nNOTES APP TESTING COMPLETED.");
}
