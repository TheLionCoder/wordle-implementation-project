use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::num::NonZeroUsize;
use flate2::bufread::GzDecoder;
use rayon::prelude::*;

const DICTIONARY: &str = include_str!("../../assets/data/dictionary.txt");



fn main() {
    let files: Vec<String> = std::env::args().skip(1).collect();
    let words: HashMap<&[u8], usize, _> = files
        .into_par_iter()
        .map(|file: String| {
            let file: File = File::open(&file)
                .unwrap_or_else(|e| panic!("could not open file '{}': {}", file, e));
            let file: BufReader<File> = BufReader::new(file);
            let file: GzDecoder<BufReader<File>> = GzDecoder::new(file);
            let mut file: BufReader<GzDecoder<BufReader<File>>> = BufReader::new(file);
            let mut words: HashMap<&[u8], usize> = DICTIONARY.lines().map(
                |word| (word.as_bytes(), 0)).collect();
            let mut line: Vec<u8> = Vec::new();
            loop {
                line.clear();
                if file.read_until(b'\n', &mut line)
                    .expect("reading from stdin should be okay") == 0
                {
                    break;
                }

                let mut fields = line.split_mut(|&c| c == b'\t');
                let word: &mut [u8] = fields.next().expect("every line should have a word");
                let word: &mut [u8] = if let Some(w) = word.splitn_mut(2, |&c| c == b'_').next() {
                    w
                } else {
                    word
                };

                if word.len() != 5 {
                    line.clear();
                    continue;
                }
                if !word.iter().all(|c| matches!(c, b'a'..=b'z' | b'A'..=b'Z')) {
                    continue;
                }

                word.make_ascii_lowercase();
                if let Some(accum) = words.get_mut(&*word) {
                    let count: usize = fields
                        .map(|field| {
                            let mut columns = field.split(|&c| c == b',');
                            let count: &[u8] = columns.nth(1).expect("every row has three fields");
                            let mut value: usize= 0;
                            let mut dec: usize= 1;
                            for &digit in count.iter().rev() {
                                assert!(matches!(digit, b'0'..=b'9'));
                                let digit: u8 = digit - b'0';
                                value += digit as usize * dec;
                                dec *= 10;
                            }
                            value
                        })
                        .sum();
                    *accum += count;
                }
            }
            words
        })
        .reduce(HashMap::new, |mut map1, map2| {
            for (word, count) in map2 {
                *map1.entry(word).or_insert(0) += count;
            }
            map1
        });

    for word in DICTIONARY.lines() {
        let count: usize = words
            .get(word.as_bytes())
            .copied()
            .and_then(NonZeroUsize::new)
            .map(|value| value.into())
            .unwrap_or(1);
        println!("{} {}", word, count);
    }

}

