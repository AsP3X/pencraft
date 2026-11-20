//! The task list of a long job: the `plan_tasks` and `finish_task` tools, and the list they keep.
//!
//! A job of many steps (a whole screen, several screens, a redesign) is split into tasks before the first
//! layer is made. The list stands at the top of the chat panel, where the user follows it. The tasks are done
//! in their order, one at a time: the task at work is finished with `finish_task`, and only then is the next
//! one at work. No task can be skipped or finished out of its turn.
//!
//! A task can turn up more work: checking the result finds something to fix. `add_tasks` puts new tasks right
//! after the task at work. When every task is finished the list leaves the top of the panel: a moment later it
//! is filed into the conversation, at the place where it was finished, and the next long job starts a new list.

use std::time::{Duration, Instant};

use serde_json::Value;

/// The tool that makes the list, or changes the tasks that have not started.
pub const PLAN: &str = "plan_tasks";
/// The tool that finishes the task at work.
pub const FINISH: &str = "finish_task";
/// The tool that adds tasks after the task at work.
pub const ADD: &str = "add_tasks";
/// How long a list stays at the top of the panel after its last task is finished, before it is filed into the
/// conversation: long enough for the last tick to be seen.
pub const FILED_AFTER: Duration = Duration::from_millis(1500);
/// The fewest tasks a list has: one task is no list.
pub const FEWEST: usize = 2;
/// The most tasks a list has.
pub const MOST: usize = 12;
/// The most letters of a task that are shown.
const MOST_TASK: usize = 80;
/// How many changes of the design a run makes without a list before it is told to make one.
// Human: a small request is a change or two and a look at the result; a run that is still changing the design
// after this many calls is a long job, whatever it looked like at the start.
const LOOSE_CHANGES: u32 = 6;

/// What a caller outside the app's own chat is told.
pub const NO_CHAT: &str = "Only the assistant in Pencraft's own chat keeps a task list. Work without one.";
/// What a run that changes the design many times without a list is told, once.
pub const ADVICE: &str = "This has become a long job. Split what is left into tasks with plan_tasks now (a few words each, in the order you will do them), then work on one task at a time and call finish_task after each: the user follows the list above the chat.";

/// How far a task has got.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// Its turn has not come.
    Waiting,
    /// It is the task at work.
    Doing,
    /// It is finished.
    Done,
}

/// The task list of one chat.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Tasks {
    /// The tasks in the order they are done.
    list: Vec<String>,
    /// How many of them are finished: the tasks before this place.
    done: usize,
    /// The run that worked on the list was stopped or broke off; the task at work is marked until work goes on.
    halted: bool,
    /// How many lists this chat has had: a new number is a new list, which the panel pours in.
    made: u64,
    /// How many times the design was changed in this run while there was no list to work through.
    loose: u32,
    /// A list was made while this message was being answered: the run is not told to make one.
    planned: bool,
    /// The tasks added last with [`ADD`]: where the first of them stands, how many they are, and the number of
    /// that addition among the list's additions (0 before the first).
    fresh: (usize, usize),
    grown: u64,
    /// When the last task was finished, until the list is filed into the conversation.
    finished: Option<Instant>,
    /// How many lists of this chat were filed into the conversation.
    handed: u64,
}

/// `text` on one line, cut after [`MOST_TASK`] letters.
fn tidy(text: &str) -> String {
    let line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if line.chars().count() > MOST_TASK { line.chars().take(MOST_TASK - 1).chain(['…']).collect() } else { line }
}

/// The tasks a call of [`PLAN`] with `input` names.
// Human: a model sometimes writes a task as an object with a title; that title is taken.
fn named(input: &Value) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for item in input.get("tasks").and_then(Value::as_array).into_iter().flatten() {
        let words = item.as_str().or_else(|| ["task", "title", "text", "name"].iter().find_map(|key| item.get(key).and_then(Value::as_str)));
        let Some(text) = words.map(tidy).filter(|text| !text.is_empty()) else { continue };
        if !out.contains(&text) {
            out.push(text);
        }
    }
    out
}

impl Tasks {
    /// A list of six tasks of which `done` are finished, for a picture of the chat panel (`--sidebar tasks`).
    #[must_use]
    pub fn sample(done: usize) -> Tasks {
        let list = [
            "Look at the current settings window",
            "Set up colours and type styles",
            "Build the navigation column",
            "Lay out the General page",
            "Lay out the Models page",
            "Check the result against the brief",
        ];
        Tasks { list: list.map(str::to_owned).to_vec(), done: done.min(list.len()), made: 1, ..Tasks::default() }
    }

    /// The tasks with how far each has got, in their order.
    pub fn each(&self) -> impl Iterator<Item = (&str, State)> {
        self.list.iter().enumerate().map(|(index, text)| {
            let state = match index.cmp(&self.done) {
                std::cmp::Ordering::Less => State::Done,
                std::cmp::Ordering::Equal => State::Doing,
                std::cmp::Ordering::Greater => State::Waiting,
            };
            (text.as_str(), state)
        })
    }

    /// How many tasks the list has.
    #[must_use]
    pub fn len(&self) -> usize {
        self.list.len()
    }

    /// Whether there is no list.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    /// How many tasks are finished.
    #[must_use]
    pub fn done(&self) -> usize {
        self.done
    }

    /// Whether there is a list and every task of it is finished.
    #[must_use]
    pub fn all_done(&self) -> bool {
        !self.list.is_empty() && self.done == self.list.len()
    }

    /// Whether there is a list with a task still to do.
    #[must_use]
    pub fn open(&self) -> bool {
        self.done < self.list.len()
    }

    /// The task at work, if one is.
    #[must_use]
    pub fn current(&self) -> Option<&str> {
        self.list.get(self.done).map(String::as_str)
    }

    /// Whether the run that worked on the list was stopped or broke off before the task at work was finished.
    #[must_use]
    pub fn halted(&self) -> bool {
        self.halted && self.open()
    }

    /// The number of this list among the chat's lists; it changes when a new list is made.
    #[must_use]
    pub fn made(&self) -> u64 {
        self.made
    }

    /// `plan_tasks`: makes the list, or gives the tasks that have not started a new wording and order.
    ///
    /// # Errors
    /// What is wrong with the input, said so that the model can put it right.
    // Human: a list that is still being worked through keeps what is finished and the task at work; only what
    // has not started can change. The user watched those tasks being done and they stay as they were.
    pub fn plan(&mut self, input: &Value) -> Result<String, String> {
        let tasks = named(input);
        if self.open() {
            let kept = self.done + 1;
            if tasks.is_empty() {
                return Err("Give `tasks`: the tasks still to do after the one at work, in the order you will do them.".to_owned());
            }
            if kept + tasks.len() > MOST {
                return Err(format!("A list has at most {MOST} tasks; {kept} are finished or at work, so give at most {} more.", MOST - kept));
            }
            self.list.truncate(kept);
            self.list.extend(tasks);
            self.halted = false;
            return Ok(format!("The tasks that had not started are replaced. {}", self.told()));
        }
        if tasks.len() < FEWEST {
            return Err(format!("Give `tasks`: {FEWEST} to {MOST} short tasks in the order you will do them. A job of one step needs no list."));
        }
        if tasks.len() > MOST {
            return Err(format!("A list has at most {MOST} tasks. Put small steps together until it has."));
        }
        self.list = tasks;
        self.done = 0;
        self.halted = false;
        self.loose = 0;
        self.planned = true;
        self.finished = None;
        self.made += 1;
        Ok(format!(
            "The user now sees the list. {} Call finish_task when it is finished; do the tasks in this order and don't work ahead.",
            self.told()
        ))
    }

    /// The tasks added last: the number of that addition (0 before the first), where its first task stands,
    /// and how many they are.
    #[must_use]
    pub fn added(&self) -> (u64, usize, usize) {
        (self.grown, self.fresh.0, self.fresh.1)
    }

    /// How many lists of this chat were filed into the conversation.
    #[must_use]
    pub fn handed(&self) -> u64 {
        self.handed
    }

    /// `add_tasks`: puts new tasks right after the task at work; the tasks that were waiting come after them.
    ///
    /// # Errors
    /// That no task is at work, or what is wrong with the input.
    // Human: work turns up while a task is done: a check finds a button that is cut off. The fix is a task of
    // its own, next in line, and the user sees it join the list.
    pub fn add(&mut self, input: &Value) -> Result<String, String> {
        if !self.open() {
            return Err("No task is at work, so there is nothing to add after. Make a list with plan_tasks.".to_owned());
        }
        let tasks: Vec<String> = named(input).into_iter().filter(|task| !self.list.contains(task)).collect();
        if tasks.is_empty() {
            return Err("Give `tasks`: the new tasks, a few words each. A task the list already has is not added again.".to_owned());
        }
        if self.list.len() + tasks.len() > MOST {
            let room = MOST - self.list.len();
            return Err(format!(
                "A list has at most {MOST} tasks and this one has {}; add at most {room}, or put small fixes into one task.",
                self.list.len()
            ));
        }
        let at = self.done + 1;
        self.fresh = (at, tasks.len());
        self.grown += 1;
        self.list.splice(at..at, tasks);
        Ok(format!("Added after the task at work. {}", self.told()))
    }

    /// Whether the list has been finished for long enough at `now` to be filed into the conversation.
    #[must_use]
    pub fn due(&self, now: Instant) -> bool {
        self.all_done() && self.finished.is_none_or(|finished| now.saturating_duration_since(finished) >= FILED_AFTER)
    }

    /// Takes the finished list out, to be filed into the conversation: its tasks. The chat has no list then.
    pub fn hand_over(&mut self) -> Vec<String> {
        if !self.all_done() {
            return Vec::new();
        }
        self.done = 0;
        self.finished = None;
        self.halted = false;
        self.handed += 1;
        std::mem::take(&mut self.list)
    }

    /// `finish_task`: finishes the task at work; the next one is then at work.
    ///
    /// # Errors
    /// That there is no task at work, or that the task named is not the one whose turn it is.
    pub fn finish(&mut self, input: &Value) -> Result<String, String> {
        let Some(current) = self.current().map(str::to_owned) else {
            return Err(if self.list.is_empty() {
                "There is no task list. Make one with plan_tasks first.".to_owned()
            } else {
                "Every task of the list is already finished.".to_owned()
            });
        };
        let turn = self.done + 1;
        if let Some(named) = input.get("task").and_then(Value::as_u64)
            && usize::try_from(named).ok() != Some(turn)
        {
            return Err(format!(
                "Task {named} is not the one at work. The tasks are done in their order: finish task {turn} (\"{current}\") first, and call finish_task with `task` {turn} when it is."
            ));
        }
        self.done = turn;
        self.halted = false;
        if self.all_done() {
            self.finished = Some(Instant::now());
        }
        Ok(match self.current() {
            Some(_) => format!("Task {turn} is finished. {}", self.told()),
            None => format!("Task {turn} is finished, and with it all {} tasks. Tell the user in a few sentences what you made.", self.list.len()),
        })
    }

    /// What the model is told about where the list stands; empty without a task at work.
    #[must_use]
    pub fn told(&self) -> String {
        let Some(current) = self.current() else { return String::new() };
        let after = self.list.len() - self.done - 1;
        let rest = match after {
            0 => "It is the last one.".to_owned(),
            1 => "1 task comes after it.".to_owned(),
            more => format!("{more} tasks come after it."),
        };
        format!("At work now: task {} of {}, \"{current}\". {rest}", self.done + 1, self.list.len())
    }

    /// What a message of the user's says about the list to the assistant that takes it; empty when no task is
    /// waiting to be done.
    #[must_use]
    pub fn reminder(&self) -> String {
        if !self.open() {
            return String::new();
        }
        format!("Your task list is not finished. {} Go on with it unless the user asks for something else.", self.told())
    }

    /// The run was stopped or broke off: the task at work is marked until work on the list goes on.
    pub fn halt(&mut self) {
        self.halted = self.open();
    }

    /// A new message is being answered: nothing is marked any more, and the count of loose changes starts anew.
    pub fn resume(&mut self) {
        self.halted = false;
        self.loose = 0;
        self.planned = false;
    }

    /// The design was changed: what to tell the model about making a list, the one time it is due.
    // Human: while a list is being worked through nothing is said; neither after one was finished in this run.
    pub fn changed(&mut self) -> Option<&'static str> {
        if !self.list.is_empty() || self.planned {
            return None;
        }
        self.loose += 1;
        (self.loose == LOOSE_CHANGES).then_some(ADVICE)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn states(tasks: &Tasks) -> Vec<State> {
        tasks.each().map(|(_, state)| state).collect()
    }

    #[test]
    fn tasks_are_done_one_at_a_time_in_their_order() {
        let mut tasks = Tasks::default();
        assert!(tasks.finish(&json!({})).unwrap_err().contains("plan_tasks"));
        let said = tasks.plan(&json!({ "tasks": ["Look at  the window", { "title": "Build the navigation" }, "Check the result", ""] })).unwrap();
        assert!(said.contains("task 1 of 3, \"Look at the window\"") && said.contains("2 tasks come after it"), "{said}");
        assert_eq!(states(&tasks), [State::Doing, State::Waiting, State::Waiting]);
        assert_eq!((tasks.made(), tasks.current()), (1, Some("Look at the window")));
        // A task that is not at work can't be finished: the one before it comes first.
        let refused = tasks.finish(&json!({ "task": 2 })).unwrap_err();
        assert!(refused.contains("finish task 1"), "{refused}");
        assert_eq!(tasks.done(), 0);
        assert!(tasks.finish(&json!({ "task": 1 })).unwrap().contains("task 2 of 3"));
        assert_eq!(states(&tasks), [State::Done, State::Doing, State::Waiting]);
        // Without a number the task at work is meant.
        assert!(tasks.finish(&json!({})).unwrap().contains("It is the last one"));
        let last = tasks.finish(&json!({ "task": 3 })).unwrap();
        assert!(last.contains("all 3 tasks") && tasks.all_done() && !tasks.open(), "{last}");
        assert!(tasks.finish(&json!({})).unwrap_err().contains("already finished"));
    }

    #[test]
    fn a_list_at_work_keeps_what_is_done_and_takes_new_tasks_for_the_rest() {
        let mut tasks = Tasks::default();
        tasks.plan(&json!({ "tasks": ["One", "Two", "Three"] })).unwrap();
        tasks.finish(&json!({})).unwrap();
        let said = tasks.plan(&json!({ "tasks": ["Three, reworded", "Four"] })).unwrap();
        assert!(said.contains("task 2 of 4, \"Two\""), "{said}");
        assert_eq!(tasks.each().map(|(text, _)| text).collect::<Vec<_>>(), ["One", "Two", "Three, reworded", "Four"]);
        // The same list, not a new one: the panel doesn't pour it in again.
        assert_eq!((tasks.made(), tasks.done()), (1, 1));
        let many: Vec<String> = (0..MOST).map(|n| format!("Task {n}")).collect();
        assert!(tasks.plan(&json!({ "tasks": many })).unwrap_err().contains("at most"));
        // A finished list that was not filed yet gives way to a new one.
        for _ in 0..3 {
            tasks.finish(&json!({})).unwrap();
        }
        tasks.plan(&json!({ "tasks": ["Again", "And again"] })).unwrap();
        assert_eq!((tasks.made(), tasks.done(), tasks.len()), (2, 0, 2));
    }

    #[test]
    fn work_that_turns_up_is_added_after_the_task_at_work() {
        let mut tasks = Tasks::default();
        assert!(tasks.add(&json!({ "tasks": ["Fix the button"] })).unwrap_err().contains("plan_tasks"));
        tasks.plan(&json!({ "tasks": ["Build", "Check the result", "Export"] })).unwrap();
        tasks.finish(&json!({})).unwrap();
        assert_eq!(tasks.added(), (0, 0, 0));
        let said = tasks.add(&json!({ "tasks": ["Fix the cut-off button", "Check the result", "Check again"] })).unwrap();
        // The new tasks stand right after the one at work; one the list has is not added twice.
        assert_eq!(
            tasks.each().map(|(text, _)| text).collect::<Vec<_>>(),
            ["Build", "Check the result", "Fix the cut-off button", "Check again", "Export"]
        );
        assert!(said.contains("task 2 of 5, \"Check the result\"") && said.contains("3 tasks come after it"), "{said}");
        assert_eq!((tasks.added(), tasks.made(), tasks.done()), ((1, 2, 2), 1, 1));
        assert!(tasks.add(&json!({ "tasks": ["Check again"] })).unwrap_err().contains("`tasks`"));
        let many: Vec<String> = (0..MOST).map(|n| format!("Fix {n}")).collect();
        assert!(tasks.add(&json!({ "tasks": many })).unwrap_err().contains("at most"));
    }

    #[test]
    fn a_finished_list_is_filed_after_a_moment_and_the_chat_has_none_then() {
        let mut tasks = Tasks::default();
        tasks.plan(&json!({ "tasks": ["One", "Two"] })).unwrap();
        tasks.finish(&json!({})).unwrap();
        let now = Instant::now();
        // A list with a task to do is not filed, and nothing is taken out of it.
        assert!(!tasks.due(now + FILED_AFTER * 2) && tasks.hand_over().is_empty() && tasks.len() == 2);
        tasks.finish(&json!({})).unwrap();
        assert!(!tasks.due(now) && tasks.due(now + FILED_AFTER * 2));
        assert_eq!(tasks.hand_over(), ["One", "Two"]);
        assert!(tasks.is_empty() && !tasks.all_done() && tasks.handed() == 1 && !tasks.due(now + FILED_AFTER * 2));
        // The run that finished a list is not told to make one; the next list is a new one.
        assert!((0..20).all(|_| tasks.changed().is_none()));
        tasks.plan(&json!({ "tasks": ["Three", "Four"] })).unwrap();
        assert_eq!((tasks.made(), tasks.handed(), tasks.done()), (2, 1, 0));
    }

    #[test]
    fn a_list_needs_tasks_and_not_too_many() {
        let mut tasks = Tasks::default();
        assert!(tasks.plan(&json!({ "tasks": ["Only one"] })).unwrap_err().contains("needs no list"));
        assert!(tasks.plan(&json!({})).unwrap_err().contains("`tasks`"));
        let many: Vec<String> = (0..=MOST).map(|n| format!("Task {n}")).collect();
        assert!(tasks.plan(&json!({ "tasks": many })).unwrap_err().contains("at most"));
        assert!(tasks.is_empty() && tasks.reminder().is_empty());
        // The same task twice is one task; a long one is cut.
        tasks.plan(&json!({ "tasks": ["Same", "Same", "x".repeat(200)] })).unwrap();
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks.each().nth(1).map(|(text, _)| text.chars().count()), Some(MOST_TASK));
    }

    #[test]
    fn a_stopped_run_marks_the_task_at_work_until_work_goes_on() {
        let mut tasks = Tasks::default();
        tasks.halt();
        assert!(!tasks.halted());
        tasks.plan(&json!({ "tasks": ["One", "Two"] })).unwrap();
        tasks.halt();
        assert!(tasks.halted() && tasks.reminder().contains("task 1 of 2"));
        tasks.resume();
        assert!(!tasks.halted());
        tasks.halt();
        tasks.finish(&json!({})).unwrap();
        assert!(!tasks.halted());
    }

    #[test]
    fn a_run_that_keeps_changing_the_design_without_a_list_is_told_once_to_make_one() {
        let mut tasks = Tasks::default();
        let told: Vec<bool> = (0..LOOSE_CHANGES + 3).map(|_| tasks.changed().is_some()).collect();
        assert_eq!(told.iter().filter(|told| **told).count(), 1);
        assert!(told[LOOSE_CHANGES as usize - 1]);
        // The next message counts anew; with a list nothing is said.
        tasks.resume();
        tasks.plan(&json!({ "tasks": ["One", "Two"] })).unwrap();
        assert!((0..20).all(|_| tasks.changed().is_none()));
    }
}
