#![allow(clippy::type_complexity)]
#![allow(clippy::blocks_in_conditions)]

mod solver;

use std::{borrow::Cow, collections::HashSet, num::NonZeroU8};

pub const MAX_MASK_ENUM: usize = 5 * 5 * 5 * 5 * 5;

pub trait Guesser {
    fn guess(&mut self, history: &[Guess]) -> String;
    fn finish(&self, _guesses: usize) {}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Correctness {
    /// Green
    Correct,
    /// Yellow
    Misplaced,
    /// Gray
    Wrong
}

impl Correctness {
    fn is_misplaced(letter: u8, answer: &str, used: &mut [bool; 5]) -> bool {
        answer.bytes().enumerate().any(|(i, a)| {
            if a == letter && !used[i] {
                used[i] = true;
                return true;
            }
            false
        })
    }

    pub fn compute(answer: &str, guess: &str) -> [Self; 5] {
        assert_eq!(answer.len(), 5);
        assert_eq!(guess.len(), 5);

        let mut correctness: [Correctness; 5] = [Correctness::Wrong; 5];
        let answer_bytes: &[u8] = answer.as_bytes();
        let guess_bytes: &[u8] = guess.as_bytes();
        let mut misplaced = [0_u8; (b'z' - b'a' +1) as usize];

        // find corrected letters
        for ((&answer, &guess), c) in answer_bytes.iter()
            .zip(guess_bytes).zip(correctness.iter_mut()) {
           if answer == guess {
               *c = Correctness::Correct
           } else {
               // if the letter does not match, count it as misplacing
               misplaced[(answer - b'a') as usize] += 1;
           }
        }

        for (&guess, c) in guess_bytes.iter().zip(correctness.iter_mut()) {
            if *c == Correctness::Wrong && misplaced[(guess - b'a') as usize] > 0 {
                *c = Correctness::Misplaced;
                misplaced[(guess - b'a' ) as usize] += 1
            }
        }
        correctness
    }
}


#[derive( Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct PacketCorrectness(NonZeroU8);

impl From<[Correctness; 5]> for PacketCorrectness {
    fn from(correct: [Correctness; 5]) -> Self {
        let packed = correct.iter().fold(0, |acc, c| {
            acc * 3 +
            match c {
                Correctness::Correct => 0,
                Correctness::Misplaced => 1,
                Correctness::Wrong => 2
            }
        });
        Self(NonZeroU8::new(packed + 1).unwrap())
    }
}

impl From<PacketCorrectness> for u8 {
    fn from(this: PacketCorrectness) -> Self {
        this.0.get() -1
    }
}

pub struct Wordle {
    dictionary: HashSet<&'static str>
}

pub struct Guess<'a> {
    pub word: Cow<'a, str>,
    pub mask: [Correctness; 5]
}

impl Guess<'_> {
    pub fn matches(&self, word: &str) -> bool {
        assert_eq!(word.len(), 5);
        assert_eq!(self.word.len(), 5);
        let mut used: [bool; 5] = [false; 5];

        // check corrected letters
        for (i, (a, g)) in word.bytes().zip(self.word.bytes()).enumerate() {
            if a == g {
                if self.mask[i] != Correctness::Correct {
                    return false;
                }
                used[i] = true;
            } else if self.mask[i] == Correctness::Correct {
                    return false;
                }
            }
            // check misplaced letters
            for (g, e) in self.word.bytes().zip(self.mask.iter()) {
                if *e == Correctness::Correct {
                    continue;
                }
                if Correctness::is_misplaced(g, word, &mut used) != (*e == Correctness::Misplaced) {
                    return false;
                }
            }
            true
        }
    }

impl Default for Wordle {
    fn default() -> Self {
        Self::new()
    }
}

impl Wordle {
    pub fn new() -> Self {
        Self {
            dictionary: HashSet::from_iter(DICTIONARY.lines().iter().copied()
                .map(|(word, _)| word))
        }
    }

    pub fn play<G: Guesser>(&self, answer: &'static str, mut guesser: G) -> Option<usize> {
        let mut history: Vec<Guess> = Vec::new();

        // Allow more than six guesses for distribution purposes
        for i in 1..=32 {
            let guess: String = guesser.guess(&history);
            if guess == answer {
                guesser.finish(i);
                return Some(i);
            }
            assert!(
                self.dictionary.contains(&*guess),
                "guess '{}' is not in the dictionary",
                guess
            );
            let correctness: [Correctness; 5] = Correctness::compute(answer, &guess);
            history.push(Guess {
                word: Cow::Owned(guess),
                mask: correctness,
            });
        }
        None
    }
}

#[cfg(test)]
macro_rules! guessers {
    (|$history:ident | $impl:block) => {{
        struct G;
        $impl $create::Guesser for G {
            fn guess(&mut self, $history: &[Guess]) -> String {
                $impl
            }
        }
        G
    }};
}

#[cfg(test)]
macro_rules! mask {
    (C) => {$crate::Correctness::Correct};
    (M) => {$crate::Correctness::Misplaced};
    (W) => {$crate::Correctness::Wrong};
    ($($c:tt)+) => {
        $(mask!($c)), +
    }
}

#[cfg(test)]
mod guess_matcher {
    use crate::Guess;
    use std::borrow::{Borrow, Cow};

    macro_rules! check {
        ($prev:literal + [$($mask:tt)+] allows $next:literal) => {
            assert!(Guess {
                word: Cow::Borrowed($prev),
                mask: mask![$($mask ) +]
            }
            .matches($next)
            );
            assert_eq!($crate::Correctness::compute($next, $prev), mask![$($mask) +]);
        };
        ($prev:literal + [$($mask:tt)+] disallows $next:literal) => {
            assert!(!Guess {
                word: Cow::Borrowed($prev),
                mask: mask![$($mssk)+]
            }
            .matches($next));
            assert_ne!($crate::Correctness:compute($next, $prev), $mask![$($mask )+]);
        }
    }

}

#[test]
fn from_jon() {
    check!("abcde" + [C C C C C] allows "abcde");
    check!("abcdf" + [C C C C C] disallows "abcde");
    check!("abcde" + [W W W W W] allows "fghij");
    check!("abcde" + [M M M M M] allows "eabcd");
    check!("baaa" + [W C M W W] allows "aaccc");
    check!("baaa" + [W C M W W] disallows "caacc");
}