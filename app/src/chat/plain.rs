//! Answering with any command-line agent that takes a prompt and prints its answer.
//!
//! The program is run once per message with the arguments the user gave it in the settings. It keeps no
//! session here, so every message is told what was said before. It reaches the document like any outside AI
//! client: through `pencraft mcp`, which the user adds to the program's own MCP servers once.

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use super::Event;
use super::code::{collect_errors, folder, tail, windowless};
use crate::app::lock;

/// Where the message goes in the arguments.
const PROMPT: &str = "{prompt}";
/// Where the model's name goes in the arguments.
const MODEL: &str = "{model}";

/// One message to answer.
pub struct Job<'a> {
    /// Where the program is.
    pub program: &'a Path,
    /// Its arguments as the user wrote them, with `{prompt}` and `{model}` to fill in.
    pub arguments: &'a str,
    /// The model it is asked to use; empty for its own default.
    pub model: &'a str,
    /// What the user wants kept to in every conversation (the settings' custom instructions).
    pub instructions: &'a str,
    /// The user's message, with the app's note before it.
    pub prompt: &'a str,
}

/// The arguments in `template`, split at spaces (quotes keep words together) with `{prompt}` and `{model}`
/// filled in. Returns them and whether the prompt went into one; without `{prompt}` it goes to the program's
/// input instead.
// Human: the prompt is filled in after the splitting, so it stays one argument whatever it contains.
#[must_use]
pub fn arguments(template: &str, prompt: &str, model: &str) -> (Vec<String>, bool) {
    let mut words = Vec::new();
    let mut word = String::new();
    let (mut quote, mut open) = (None, false);
    for letter in template.chars() {
        match (quote, letter) {
            (Some(mark), _) if letter == mark => quote = None,
            (None, '"' | '\'') => {
                quote = Some(letter);
                open = true;
            }
            (None, _) if letter.is_whitespace() => {
                if open || !word.is_empty() {
                    words.push(std::mem::take(&mut word));
                }
                open = false;
            }
            _ => word.push(letter),
        }
    }
    if open || !word.is_empty() {
        words.push(word);
    }
    let placed = words.iter().any(|word| word.contains(PROMPT));
    (words.into_iter().map(|word| word.replace(PROMPT, prompt).replace(MODEL, model)).collect(), placed)
}

/// Runs the program for `job`, telling the chat what it prints. Returns when the program has ended.
// Agent: BLOCKS until the program ends; `child` holds it meanwhile so the chat can end it; SPAWNS a thread
// that collects the program's error output.
pub fn run(job: &Job<'_>, stop: &AtomicBool, child: &Mutex<Option<Child>>, tell: &dyn Fn(Event) -> bool) {
    let prompt = format!("{}\n\n{}", super::brief(false, job.instructions), job.prompt);
    let (arguments, placed) = arguments(job.arguments, &prompt, job.model);
    let mut command = Command::new(job.program);
    command.args(arguments);
    windowless(&mut command);
    if let Some(folder) = folder() {
        command.current_dir(folder);
    }
    let started = command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn();
    let mut running = match started {
        Ok(running) => running,
        Err(error) => {
            tell(Event::Problem(format!("The program couldn't be started ({}): {error}", job.program.display())));
            return;
        }
    };
    if let Some(mut input) = running.stdin.take()
        && !placed
    {
        // A program that ended at once has closed its input; its error output says why.
        let _ = input.write_all(prompt.as_bytes());
    }
    let output = running.stdout.take();
    let errors = collect_errors(&mut running);
    *lock(child) = Some(running);
    let mut wrote = false;
    for line in output.into_iter().flat_map(|output| BufReader::new(output).lines()) {
        let Ok(line) = line else { break };
        wrote |= !line.trim().is_empty();
        if !tell(Event::Text(format!("{line}\n"))) {
            break;
        }
    }
    let status = lock(child).take().and_then(|mut running| running.wait().ok());
    let said = errors.and_then(|errors| errors.join().ok()).unwrap_or_default();
    if stop.load(Ordering::Relaxed) || (wrote && status.is_some_and(|status| status.success())) {
        return;
    }
    let said = tail(&said);
    let hint = if said.is_empty() { "Check its arguments in Settings → Agents, and that it is signed in." } else { said };
    tell(Event::Problem(format!("The program ended without an answer. {hint}")));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arguments_are_split_at_spaces_and_the_prompt_stays_one() {
        let (words, placed) = arguments(r#"exec --model {model} "--note=two words" '' {prompt}"#, "Make a \"card\" now", "m1");
        assert_eq!(words, ["exec", "--model", "m1", "--note=two words", "", "Make a \"card\" now"]);
        assert!(placed);
        // Without a place for it the prompt goes to the program's input.
        assert_eq!(arguments("  -p  ", "hi", ""), (vec!["-p".to_owned()], false));
        assert_eq!(arguments("", "hi", ""), (Vec::new(), false));
    }

    // Human: the test's stand-in program is the shell, which only Unix has at this path; Windows is covered by
    // the test of the arguments above, which is the part that differs by nothing.
    #[cfg(unix)]
    #[test]
    fn a_program_prints_its_answer_and_a_failing_one_says_why() {
        let told = std::cell::RefCell::new(Vec::new());
        let tell = |event: Event| {
            told.borrow_mut().push(format!("{event:?}"));
            true
        };
        let (stop, child) = (AtomicBool::new(false), Mutex::new(None));
        let job = |arguments: &'static str| Job { program: Path::new("/bin/sh"), arguments, model: "", instructions: "", prompt: "Hello" };
        // The message as an argument, and as the program's input: the brief first, the user's words last.
        for arguments in [r#"-c 'printf "%s" "$0"' {prompt}"#, "-c cat"] {
            run(&job(arguments), &stop, &child, &tell);
            let said = told.borrow().concat();
            assert!(said.starts_with(r#"Text("You are the design assistant"#) && said.ends_with(r#"Text("Hello\n")"#), "{arguments}");
            told.borrow_mut().clear();
        }
        run(&job(r#"-c 'echo "not signed in" >&2; exit 3'"#), &stop, &child, &tell);
        assert_eq!(*told.borrow(), [r#"Problem("The program ended without an answer. not signed in")"#]);
    }
}
