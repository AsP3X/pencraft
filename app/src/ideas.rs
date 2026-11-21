//! Ideas for a design to ask for, offered in an empty chat: a click puts one into the message box.
//!
//! There are many more than fit on the panel. Each empty chat shows a handful picked at random, and the user
//! can ask for another handful, so the same six sentences are not what every chat starts with.

/// How many ideas an empty chat shows at a time.
pub const SHOWN: usize = 6;

/// Every idea there is.
// Human: they are spread over what Quill is asked for: landing pages and sites, phone and tablet apps,
// dashboards and tools, components and kits, posters and print, logos, slides, charts, and plans and
// comparisons. Each names a look or a mood as well as a thing, since "a booking app" alone gives the
// assistant nothing to make it particular.
pub const IDEAS: [&str; 106] = [
    "Plan a car-free weekend in Kyoto for two, with a rainy-day backup",
    "Design a retro-futuristic landing page for a private space travel company",
    "Design a vintage ticketing site for an independent arthouse cinema",
    "Design a playful ordering app for matcha cakes",
    "Design a soft neumorphic meditation app inspired by Nordic design",
    "Design a high-contrast realtime trading terminal",
    // Landing pages and sites.
    "Design a landing page for a bakery that only sells sourdough, in warm flour tones",
    "Design a one-page site for a wedding photographer who shoots on film",
    "Design a bold landing page for a climbing gym in a former warehouse",
    "Design a quiet, bookish homepage for an independent publisher of translated novels",
    "Design a product page for a mechanical keyboard with swappable switches",
    "Design a landing page for a night train between Vienna and Rome",
    "Design a brutalist portfolio site for an architecture student",
    "Design a pricing page with three plans for a tool that transcribes meetings",
    "Design a site for a family-run olive oil farm in Crete, with a shop",
    "Design a landing page for an open-source database, written for engineers",
    "Design a festival site for three days of electronic music on a Baltic island",
    "Design a calm homepage for a hospice that reads as warm, not clinical",
    "Design a landing page for a sailing school on Lake Garda",
    "Design an editorial long read about deep-sea mining, with pull quotes and maps",
    "Design a waitlist page for a notebook app that works without the internet",
    "Design a site for a ceramics studio that sells wheel-throwing classes",
    "Design a campaign page for a city that wants more people to cycle",
    "Design a documentation homepage for a payments API, with a quick start",
    "Design a landing page for a board game cafe with a weekly tournament",
    // Phone and tablet apps.
    "Design an iPhone app for tracking houseplants and when to water them",
    "Design an Android app for splitting a restaurant bill among friends",
    "Design a sleep tracker for iPhone in deep blues, with a morning summary",
    "Design a language app that teaches ten words a day with small drawings",
    "Design an iPad app for a sommelier to manage a restaurant's wine cellar",
    "Design a running app that shows one number at a time, large",
    "Design a banking app for teenagers and their first allowance",
    "Design an app for finding and reviewing public swimming spots in rivers",
    "Design a recipe app that scales the ingredients to the number of guests",
    "Design a mobile checkout for a florist with same-day delivery",
    "Design a podcast player with chapters and a sleep timer",
    "Design an Android app for a village library's loans and reservations",
    "Design a habit tracker that looks like a paper bullet journal",
    "Design a birdwatching app that keeps a life list with sketches",
    "Design a parking app for a hospital, for worried people in a hurry",
    "Design a weather app that says what to wear instead of the numbers",
    "Design a medication reminder for older people, with very large type",
    "Design a messaging app for a hiking club, with routes pinned to chats",
    "Design a ticket wallet for a football club's season ticket holders",
    "Design a car-sharing app for a small town with eleven cars",
    // Dashboards and tools.
    "Design a dashboard for a wind farm with forty turbines and live output",
    "Design an admin tool for a food bank to plan drivers and deliveries",
    "Design a control room screen for a city's tram network at night",
    "Design an analytics dashboard for a newsletter with ten thousand readers",
    "Design a kanban board for a film crew's shooting days",
    "Design a hotel reception screen for check-ins, rooms and housekeeping",
    "Design a monitoring dashboard for a fleet of delivery drones",
    "Design a spreadsheet-like tool for a brewery's batches and tanks",
    "Design a support inbox for a small team, with canned replies",
    "Design a clinic's appointment planner for five doctors over one week",
    "Design a dashboard for a marathon organiser on race day",
    "Design a settings window for a desktop music player, in three sections",
    "Design a code review screen that puts the discussion beside the diff",
    "Design a warehouse picking screen for a handheld scanner",
    "Design a dashboard for a school's energy use, made for the pupils to read",
    "Design a timeline editor for a podcast with three tracks",
    "Design an energy dashboard for a home with solar panels and a battery",
    // Components and kits.
    "Design a set of buttons, inputs and toggles for a dark developer tool",
    "Design a date picker that handles ranges and works with one thumb",
    "Design a pricing table component with a monthly and yearly switch",
    "Design a set of empty states with small line drawings for a notes app",
    "Design a navigation bar that folds from desktop to phone",
    "Design a data table with sorting, filters and a row that expands",
    "Design a set of toast and alert messages in four levels of urgency",
    "Design an onboarding flow of four screens for a budgeting app",
    "Design a comment thread component with replies and reactions",
    "Design a file upload component with progress, errors and retry",
    "Design a liquid glass tab bar and toolbar for a photo app",
    "Design a sign-in and sign-up pair of screens with a passkey option",
    // Posters and print.
    "Design a poster for a jazz night in a cellar bar, in two colours",
    "Design a concert poster for a string quartet playing in a factory hall",
    "Design a menu for a ramen shop with six bowls and four sides",
    "Design a wine label for a natural red from the Jura",
    "Design a book cover for a novel about a lighthouse keeper's daughter",
    "Design a business card for a bicycle mechanic",
    "Design a wedding invitation with a hand-drawn map to the venue",
    "Design a poster for a film festival of silent comedies",
    "Design a flyer for a neighbourhood repair cafe on the first Sunday",
    "Design a magazine cover for an issue on the future of trains",
    "Design a record sleeve for an ambient album recorded in Iceland",
    "Design a zine spread about fermenting vegetables at home",
    // Logos and identities.
    "Design a logo for a coffee roaster called Northbound",
    "Design a logo for a law firm that wants to look less like a law firm",
    "Design a mark for a children's science museum",
    "Design a badge logo for a surf school in Portugal, with a rim of text",
    "Design a wordmark for a furniture maker who works only in ash",
    "Design a logo for a podcast about unsolved maths problems",
    "Design an app icon for a calculator that keeps a paper tape",
    "Design a crest for an amateur rowing club founded in 1908",
    // Slides, charts and explanations.
    "Design a five-slide pitch for a company that recycles fishing nets",
    "Design a title slide and an agenda for a talk on type design",
    "Design a chart of a city's rainfall over twelve months, fit for print",
    "Design a slide that explains how a heat pump works, with one diagram",
    "Design a quarterly report page with four numbers and two charts",
    "Design an infographic on how a letter travels from postbox to door",
    "Design a comparison chart of three electric cargo bikes",
    "Design a timeline of the moon landings as one wide frame",
    // Plans and comparisons.
    "Plan a two-week rail trip through Scandinavia in June as a day-by-day board",
    "Compare three note-taking apps side by side, with what each is best at",
    "Lay out a moodboard for a seaside cafe: colours, type, materials",
    "Map the screens of a food delivery app as a flow, from open to delivered",
];

/// A number that follows from `state`, which it moves on: the same start gives the same numbers.
// Source: SplitMix64, Sebastiano Vigna (prng.di.unimi.it/splitmix64.c).
fn next(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut mixed = *state;
    mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    mixed ^ (mixed >> 31)
}

/// `count` different ideas picked by `seed`: the same seed picks the same ones, in the same order.
// Human: the list is shuffled as far as needed (Fisher–Yates), so no idea shows twice in one handful.
#[must_use]
pub fn pick(seed: u64, count: usize) -> Vec<&'static str> {
    let mut all = IDEAS.to_vec();
    let mut state = seed;
    let count = count.min(all.len());
    for place in 0..count {
        let left = all.len() - place;
        let other = place + usize::try_from(next(&mut state) % (left as u64)).unwrap_or(0);
        all.swap(place, other);
    }
    all.truncate(count);
    all
}

/// A seed for [`pick`] that differs every time: the clock's nanoseconds, mixed with `salt`.
#[must_use]
pub fn fresh_seed(salt: u64) -> u64 {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |since| since.as_nanos());
    u64::try_from(now & u128::from(u64::MAX)).unwrap_or(0) ^ salt.rotate_left(17)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_idea_is_one_sentence_of_its_own() {
        let mut seen = std::collections::HashSet::new();
        for idea in IDEAS {
            assert!(seen.insert(idea.to_lowercase()), "twice: {idea}");
            // Short enough to read at a glance, and a request, not a statement.
            assert!((24..=90).contains(&idea.chars().count()), "{idea}");
            assert!(!idea.ends_with('.') && idea.chars().next().is_some_and(char::is_uppercase), "{idea}");
        }
    }

    #[test]
    fn a_handful_is_picked_without_repeats_and_the_same_seed_picks_the_same() {
        let first = pick(7, SHOWN);
        assert_eq!(first.len(), SHOWN);
        assert_eq!(first.iter().collect::<std::collections::HashSet<_>>().len(), SHOWN);
        assert_eq!(pick(7, SHOWN), first);
        assert_ne!(pick(8, SHOWN), first);
        // Asking for more than there are gives every idea once.
        assert_eq!(pick(1, 1000).iter().collect::<std::collections::HashSet<_>>().len(), IDEAS.len());
    }

    #[test]
    fn over_many_handfuls_every_idea_gets_its_turn() {
        let mut shown = std::collections::HashSet::new();
        for seed in 0..400 {
            shown.extend(pick(seed, SHOWN));
        }
        assert_eq!(shown.len(), IDEAS.len());
    }
}
