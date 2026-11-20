//! Finding a name by a few of its letters, typed in order but not side by side ("ds4p" finds
//! "deepseek-v4-pro"), and saying which letters were meant so a list can show them.

/// Where a query was found in a text.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Found {
    /// How good the find is; a higher one stands earlier in a list.
    pub score: i32,
    /// The places of the text's letters the query's letters fell on, counted in letters and rising.
    pub at: Vec<usize>,
}

/// What every letter found is worth.
const LETTER: i32 = 16;
/// What a letter is worth more when it follows the letter found before it.
const RUN: i32 = 14;
/// What a letter is worth more at the start of a word ("opus" in "claude-opus").
const WORD: i32 = 10;
/// What a letter is worth more at the start of the text.
const FIRST: i32 = 6;
/// What every letter skipped between two found ones costs.
const SKIP: i32 = 1;
/// Stands for "no find" while the best one is worked out.
const NONE: i32 = i32::MIN / 4;

/// Whether the letter at `at` starts a word of `text`: the first letter, one after a sign or a space, a
/// capital after a small letter, or a digit after a letter.
fn starts_word(text: &[char], at: usize) -> bool {
    let Some(before) = at.checked_sub(1).map(|before| text[before]) else { return true };
    let letter = text[at];
    !before.is_alphanumeric() || (before.is_lowercase() && letter.is_uppercase()) || (before.is_alphabetic() && letter.is_numeric())
}

/// The best place for one word of a query in `text`, whose letters are `small` without their capitals.
// Human: every way of laying the word's letters over the text in order is weighed; letters side by side and at
// the start of a word count for more, skipped letters for a little less.
// Agent: one pass per letter of the word; `from` remembers, for each place, the place of the letter before.
fn word(needle: &[char], text: &[char], small: &[char]) -> Option<Found> {
    let (wanted, length) = (needle.len(), text.len());
    if wanted == 0 || wanted > length {
        return None;
    }
    let worth = |at: usize| LETTER + if starts_word(text, at) { WORD } else { 0 } + if at == 0 { FIRST } else { 0 };
    let mut best: Vec<i32> = (0..length).map(|at| if small[at] == needle[0] { worth(at) } else { NONE }).collect();
    let mut from: Vec<Vec<usize>> = Vec::with_capacity(wanted);
    from.push(vec![usize::MAX; length]);
    for letter in &needle[1..] {
        let mut next = vec![NONE; length];
        let mut came = vec![usize::MAX; length];
        // The best find of the letters so far that ends at least one letter before the one in front.
        let (mut far, mut far_at) = (NONE, usize::MAX);
        for at in 1..length {
            if at >= 2 && best[at - 2] > far {
                (far, far_at) = (best[at - 2], at - 2);
            }
            far = far.saturating_sub(SKIP).max(NONE);
            if small[at] != *letter {
                continue;
            }
            let beside = best[at - 1].saturating_add(RUN);
            let (before, before_at) = if beside >= far { (beside, at - 1) } else { (far, far_at) };
            if before > NONE / 2 {
                next[at] = before + worth(at);
                came[at] = before_at;
            }
        }
        best = next;
        from.push(came);
    }
    let (mut at, score) = best.iter().copied().enumerate().rev().max_by_key(|(_, score)| *score).filter(|(_, score)| *score > NONE / 2)?;
    let mut places = vec![at];
    for came in from.iter().skip(1).rev() {
        at = came[at];
        places.push(at);
    }
    places.reverse();
    Some(Found { score, at: places })
}

/// Looks for `query` in `text`, capitals and small letters alike. Every word of the query has to be there, its
/// letters in order; a query without letters is found everywhere.
#[must_use]
pub fn find(query: &str, text: &str) -> Option<Found> {
    let letters: Vec<char> = text.chars().collect();
    let small: Vec<char> = letters.iter().map(|letter| letter.to_lowercase().next().unwrap_or(*letter)).collect();
    let mut all = Found::default();
    for part in query.split_whitespace() {
        let needle: Vec<char> = part.chars().map(|letter| letter.to_lowercase().next().unwrap_or(letter)).collect();
        let found = word(&needle, &letters, &small)?;
        all.score += found.score;
        all.at.extend(found.at);
    }
    all.at.sort_unstable();
    all.at.dedup();
    Some(all)
}

/// The `names` that `query` is found in, the best finds first: each name's place in `names` and where the
/// query's letters are in it. Without a query every name is there, in the order given.
#[must_use]
pub fn rank<Name: AsRef<str>>(query: &str, names: &[Name]) -> Vec<(usize, Vec<usize>)> {
    if query.trim().is_empty() {
        return (0..names.len()).map(|place| (place, Vec::new())).collect();
    }
    let mut found: Vec<(usize, Found)> =
        names.iter().enumerate().filter_map(|(place, name)| find(query, name.as_ref()).map(|found| (place, found))).collect();
    // Human: of two equally good finds the shorter name is the likelier one; after that the given order holds.
    found.sort_by_key(|(place, found)| (std::cmp::Reverse(found.score), names[*place].as_ref().chars().count(), *place));
    found.into_iter().map(|(place, found)| (place, found.at)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters_are_found_in_order_and_the_places_are_said() {
        assert_eq!(find("gpt4", "gpt-4o").map(|found| found.at), Some(vec![0, 1, 2, 4]));
        assert_eq!(find("GPT", "openai/gpt-oss:120b").map(|found| found.at), Some(vec![7, 8, 9]));
        assert_eq!(find("ds4p", "deepseek-v4-pro").map(|found| found.at), Some(vec![0, 4, 10, 12]));
        // Letters out of order, or not there, find nothing; neither does a query longer than the text.
        assert_eq!((find("4tpg", "gpt-4o"), find("xyz", "gpt-4o"), find("gpt-4o-mini", "gpt-4o")), (None, None, None));
        // No query is found everywhere, on no letter.
        assert_eq!(find("  ", "gpt-4o"), Some(Found::default()));
        // Every word has to be there, wherever; their places come out rising and once each.
        assert_eq!(find("pro v4", "deepseek-v4-pro").map(|found| found.at), Some(vec![9, 10, 12, 13, 14]));
        assert_eq!(find("pro v9", "deepseek-v4-pro"), None);
        // Letters outside ASCII count as one place each.
        assert_eq!(find("é4", "modèle-é4").map(|found| found.at), Some(vec![7, 8]));
    }

    #[test]
    fn the_likelier_name_comes_first() {
        let names = ["octopus-large", "claude-opus-5-5", "opus", "gpt-oss:120b", "o1-plus"];
        let ranked: Vec<&str> = rank("opus", &names).into_iter().map(|(place, _)| names[place]).collect();
        // The whole name, then the word of a name, then the letters inside a word, then letters apart.
        assert_eq!(ranked, ["opus", "claude-opus-5-5", "octopus-large", "o1-plus"]);
        // Letters side by side beat the same letters apart, and the start of a word beats its middle.
        let score = |query: &str, text: &str| find(query, text).map(|found| found.score);
        assert!(score("kimi", "kimi-k3") > score("kimi", "k-i-m-i"));
        assert!(score("k3", "kimi-k3") > score("k3", "kimik3x") && score("gl", "glm-5.3") > score("gl", "mogl"));
        // Without a query every name is there as given, with nothing to show.
        assert_eq!(rank("", &names), (0..5).map(|place| (place, Vec::new())).collect::<Vec<_>>());
        // The find lies on the word that was meant, not on the first letters that would do.
        assert_eq!(find("pro", "provider/kimi-k3.1-pro").map(|found| found.at), Some(vec![0, 1, 2]));
        assert_eq!(find("kpro", "provider/kimi-k3.1-pro").map(|found| found.at), Some(vec![14, 19, 20, 21]));
    }
}
