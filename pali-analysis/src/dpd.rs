use anyhow::{Result, bail};
use rusqlite::{Connection, OptionalExtension};
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub struct Dictionary {
    connection: Connection,
}

impl From<Connection> for Dictionary {
    fn from(connection: Connection) -> Self {
        Self { connection }
    }
}

impl Dictionary {
    /// DPD's database at `path`, read only, so that a missing file is not created.
    #[allow(clippy::missing_errors_doc)]
    pub fn open(path: &std::path::Path) -> Result<Self> {
        Ok(Self::from(Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?))
    }
}

/// The word a term belongs to, as an index term.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TermEntry {
    pub word: Option<String>,
    pub family: Option<String>,
}

struct KeyFacts {
    /// Lemmas spelled alike but from different roots, which are different words.
    multi_root: HashSet<String>,
    /// Consonant stems with a long-vowel twin: bhagavant beside bhagavā and satthar
    /// beside satthā, but not bhavant beside bhava.
    stem_twins: HashSet<String>,
    /// Lemmas in -ā that share no root with their twin in -a: saṅkhā is not saṅkha "conch".
    unrelated_long: HashSet<String>,
    /// The verb family of each word most of whose headwords are verbal.
    families: HashMap<String, String>,
    headwords: HashMap<u32, Headword>,
}

#[derive(Default)]
struct VerbalTally {
    verbal: u32,
    total: u32,
    verbal_first: Option<bool>,
    family: Option<String>,
}

impl VerbalTally {
    /// An even split goes by DPD's first listing: gata is a participle, buddha a noun.
    fn mostly_verbal(&self) -> bool {
        self.verbal * 2 > self.total || (self.verbal * 2 == self.total && self.verbal_first == Some(true))
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Word {
    key: String,
    citation: String,
    root: String,
    prior: f64,
    verbal: bool,
    grammatical: bool,
    exact_noun: bool,
    family: Option<String>,
}

struct Headword {
    lemma: String,
    pos: String,
    root_key: String,
    family_root: String,
    root_base: String,
    construction: String,
}

const MAX_SPLITS: usize = 2;

impl Dictionary {
    /// Decide the word for each term, given how often each term occurs.
    ///
    /// How common a word is depends on which terms are its, and the other way round,
    /// so the two are estimated together until they settle: each round shares every
    /// term among its readings by how common they are, and adds up the shares. A rare
    /// word then loses what it shares with a common one: paññāya is paññā, not
    /// paññāyati. A term spelled like a headword leans to it, an indeclinable above
    /// all: puna is puna, not an inflection of punāti. Equal readings go to the noun.
    #[allow(clippy::missing_errors_doc)]
    pub fn entries(&self, terms: &[(String, u64)]) -> Result<HashMap<String, TermEntry>> {
        let keys = self.key_facts()?;
        let mut readings = Vec::with_capacity(terms.len());
        for (term, _) in terms {
            readings.push(drop_tiny_lemmas(term, self.words(term, MAX_SPLITS, &keys)?));
        }

        let masses = estimate_masses(terms, &readings);
        let rank = |word: &Word| {
            let preferred = PREFERRED_ROOTS.contains(&(word.citation.as_str(), word.root.as_str()));
            (preferred, word.exact_noun, reading_weight(word, &masses), !word.verbal)
        };

        let mut entries = HashMap::with_capacity(terms.len());
        for ((term, _), words) in terms.iter().zip(&readings) {
            let mut best: Option<&Word> = None;
            for word in words {
                if best.is_none_or(|best| rank(word) > rank(best)) {
                    best = Some(word);
                }
            }
            let entry = best.map_or_else(TermEntry::default, |word| TermEntry {
                word: Some(word.key.clone()),
                family: word.family.clone(),
            });
            entries.insert(term.clone(), entry);
        }
        Ok(entries)
    }

    /// Fail if a hand-chosen reading no longer exists, as after DPD renumbers a root.
    #[allow(clippy::missing_errors_doc)]
    pub fn check_preferred_roots(&self) -> Result<()> {
        for (lemma, root) in PREFERRED_ROOTS {
            let found: i64 = self.connection.query_row(
                "SELECT count(*) FROM dpd_headwords WHERE (lemma_1 == ?1 OR lemma_1 LIKE ?1 || ' %') AND root_key == ?2",
                [lemma.replace('ṁ', "ṃ"), root.to_string()],
                |row| row.get(0),
            )?;
            if found == 0 {
                bail!("DPD has no headword {lemma} with root {root}: update PREFERRED_ROOTS");
            }
        }
        Ok(())
    }

    /// Every headword as it appears in the texts, so that a searcher can type a
    /// citation form the corpus happens not to contain.
    #[allow(clippy::missing_errors_doc)]
    pub fn lemmas(&self) -> Result<Vec<String>> {
        let mut statement = self.connection.prepare("SELECT lemma_1 FROM dpd_headwords")?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        let mut lemmas = Vec::new();
        for row in rows {
            lemmas.push(fold_niggahita(bare_lemma(&row?)));
        }
        Ok(lemmas)
    }

    /// The distinct words a term can be, in DPD's order. A term the tokenizer split
    /// from a following quotation mark (dukkhan”ti, bhavissatī”ti) has the spelling
    /// sandhi gave it, so a term with no reading tries its pre-sandhi spelling.
    fn words(&self, term: &str, splits_left: usize, keys: &KeyFacts) -> Result<Vec<Word>> {
        let words = self.spelled_words(term, splits_left, keys)?;
        if !words.is_empty() {
            return Ok(words);
        }
        match pre_sandhi_spelling(term) {
            Some(spelling) => self.spelled_words(&spelling, splits_left, keys),
            None => Ok(words),
        }
    }

    /// Facts that need all of a lemma's headwords at once.
    fn key_facts(&self) -> Result<KeyFacts> {
        let mut statement = self.connection.prepare(
            "SELECT id, lemma_1, pos, root_key, family_root, root_base, construction FROM dpd_headwords",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                Headword {
                    lemma: row.get(1)?,
                    pos: row.get(2)?,
                    root_key: row.get(3)?,
                    family_root: row.get(4)?,
                    root_base: row.get(5)?,
                    construction: row.get(6)?,
                },
            ))
        })?;
        let headwords = rows.collect::<rusqlite::Result<Vec<(u32, Headword)>>>()?;

        let bare: HashSet<String> = headwords
            .iter()
            .map(|(_, headword)| fold_niggahita(bare_lemma(&headword.lemma)))
            .collect();
        let stem_twins = bare
            .iter()
            .filter(|lemma| {
                let stem = lemma.strip_suffix("ant").or_else(|| lemma.strip_suffix("ar"));
                stem.is_some_and(|stem| bare.contains(&format!("{stem}ā")))
            })
            .cloned()
            .collect();
        let mut facts = KeyFacts {
            multi_root: HashSet::new(),
            stem_twins,
            unrelated_long: HashSet::new(),
            families: HashMap::new(),
            headwords: HashMap::new(),
        };
        let mut exact_roots: HashMap<String, HashSet<&str>> = HashMap::new();
        let mut finite: HashMap<String, bool> = HashMap::new();
        for (_, headword) in &headwords {
            let bare = fold_niggahita(bare_lemma(&headword.lemma));
            *finite.entry(bare.clone()).or_insert(true) &= FINITE_POS.contains(&headword.pos.as_str());
            exact_roots.entry(bare).or_default().insert(&headword.root_key);
        }
        facts.unrelated_long = exact_roots
            .iter()
            .filter(|(lemma, roots)| {
                let short = short_final_vowel(lemma);
                let unrelated = lemma.ends_with('ā') && exact_roots.get(&short).is_some_and(|s| s.is_disjoint(roots));
                let beside_a_verb = lemma.ends_with(['ī', 'ū']) && finite.get(&short) == Some(&true) && !finite[*lemma];
                unrelated || beside_a_verb
            })
            .map(|(lemma, _)| lemma.clone())
            .collect();
        let mut roots: HashMap<String, HashSet<&str>> = HashMap::new();
        for (_, headword) in &headwords {
            if !headword.root_key.is_empty() {
                let spelling = lemma_spelling(&headword.lemma, &facts);
                roots.entry(spelling).or_default().insert(&headword.root_key);
            }
        }
        facts.multi_root = roots.into_iter().filter(|(_, r)| r.len() > 1).map(|(lemma, _)| lemma).collect();

        let mut tallies: HashMap<String, VerbalTally> = HashMap::new();
        for (_, headword) in &headwords {
            let tally = tallies.entry(word_key(headword, &facts)).or_default();
            let verbal = VERBAL_POS.contains(&headword.pos.as_str());
            tally.total += 1;
            tally.verbal += u32::from(verbal);
            tally.verbal_first = tally.verbal_first.or(Some(verbal));
            tally.family = tally.family.take().or_else(|| verb_family(headword));
        }
        facts.families = tallies
            .into_iter()
            .filter(|(_, tally)| tally.mostly_verbal())
            .filter_map(|(key, tally)| Some((key, tally.family?)))
            .collect();
        facts.headwords = headwords.into_iter().collect();
        Ok(facts)
    }

    fn spelled_words(&self, term: &str, splits_left: usize, keys: &KeyFacts) -> Result<Vec<Word>> {
        let (ids, split) = self.lookup(term)?;
        let mut words: Vec<Word> = Vec::new();
        let mut sandhi_base = None;
        let mut headwords = Vec::with_capacity(ids.len());
        for id in ids {
            let Some(headword) = keys.headwords.get(&id) else {
                bail!("DPD's lookup names headword {id} for {term}, which does not exist");
            };
            headwords.push(headword);
        }
        let nominals: HashSet<String> = headwords
            .iter()
            .filter(|headword| NOMINAL_POS.contains(&headword.pos.as_str()))
            .map(|headword| fold_niggahita(bare_lemma(&headword.lemma)))
            .collect();
        let plain_roots: HashSet<&str> = headwords
            .iter()
            .filter(|headword| VERBAL_POS.contains(&headword.pos.as_str()) && separate_voice(headword).is_empty())
            .map(|headword| headword.root_key.as_str())
            .filter(|root| !root.is_empty())
            .collect();
        let citations: HashSet<String> =
            headwords.iter().map(|headword| fold_niggahita(bare_lemma(&headword.lemma))).collect();
        for headword in headwords {
            if is_shadowed_aorist(term, headword, &nominals)
                || is_shadowed_voice(term, headword, &plain_roots)
                || is_shadowed_twin(term, headword, &citations, keys)
            {
                continue;
            }
            if headword.pos == "sandhi" {
                sandhi_base = sandhi_base.or_else(|| sandhi_content(&headword.construction).map(str::to_string));
                continue;
            }
            add_reading(&mut words, term, headword, keys);
        }
        let is_headword = words.iter().any(|word| word.prior > 1.0);
        if splits_left > 0 && !is_headword {
            let base = sandhi_base.or_else(|| {
                let unknown = words.is_empty();
                split.as_deref().filter(|_| unknown).and_then(base_word).map(str::to_string)
            });
            if let Some(base) = base {
                let base_words = self.spelled_words(&base, splits_left - 1, keys)?;
                if !base_words.is_empty() {
                    return Ok(base_words);
                }
            }
        }
        Ok(words)
    }

    /// Headword ids and the first deconstruction. DPD's keys spell the niggahita ṃ,
    /// and `SuttaCentral`'s texts ṁ.
    fn lookup(&self, term: &str) -> Result<(Vec<u32>, Option<String>)> {
        let key = term.replace('ṁ', "ṃ");
        let row: Option<(String, String)> = self
            .connection
            .prepare_cached("SELECT headwords, deconstructor FROM lookup WHERE lookup_key == (?1)")?
            .query_one([key], |row| Ok((row.get(0)?, row.get(1)?)))
            .optional()?;
        let Some((headwords, deconstructor)) = row else {
            return Ok((Vec::new(), None));
        };
        let ids = if headwords.is_empty() { Vec::new() } else { serde_json::from_str(&headwords)? };
        let split = if deconstructor.is_empty() {
            None
        } else {
            let splits: Vec<String> = serde_json::from_str(&deconstructor)?;
            splits.into_iter().next()
        };
        Ok((ids, split))
    }
}

/// Add a headword's reading, once per word. A word takes the strongest lean of its
/// headwords, and counts as grammatical if any is, and as verbal only if all are.
fn add_reading(words: &mut Vec<Word>, term: &str, headword: &Headword, keys: &KeyFacts) {
    let key = word_key(headword, keys);
    let citation = fold_niggahita(bare_lemma(&headword.lemma));
    let prior = citation_prior(term, &citation, headword);
    let exact_noun = is_exact_noun(term, &citation, headword);
    if let Some(known) = words.iter_mut().find(|known| known.key == key) {
        known.prior = known.prior.max(prior);
        known.exact_noun |= exact_noun;
        known.grammatical |= GRAMMATICAL_POS.contains(&headword.pos.as_str());
        known.verbal &= VERBAL_POS.contains(&headword.pos.as_str());
        return;
    }
    words.push(Word {
        family: keys.families.get(&key).cloned(),
        root: headword.root_key.clone(),
        prior,
        exact_noun,
        citation,
        verbal: VERBAL_POS.contains(&headword.pos.as_str()),
        grammatical: GRAMMATICAL_POS.contains(&headword.pos.as_str()),
        key,
    });
}

fn fold_niggahita(text: &str) -> String {
    text.replace('ṃ', "ṁ")
}

/// A lemma without its numbers: "sati 1.1" is sati.
fn bare_lemma(lemma: &str) -> &str {
    lemma.split(' ').next().unwrap_or_default()
}

const MAX_ESTIMATION_ROUNDS: usize = 20_000;
/// Estimates have settled when a round moves less than this share of the total.
const SETTLED_SHARE: f64 = 1e-8;
const STARTING_MASS: f64 = 1.0;
const CITATION_PRIOR: f64 = 3.0;
const INDECLINABLE_PRIOR: f64 = 20.0;
/// Forms DPD lists as headwords that belong to a verb's paradigm.
const NOUN_POS: [&str; 3] = ["masc", "fem", "nt"];
const NOMINAL_POS: [&str; 3] = ["masc", "nt", "adj"];
const FINITE_POS: [&str; 8] = ["pr", "aor", "fut", "opt", "imp", "cond", "perf", "imperf"];
/// Words too common and short for the length rule in `drop_tiny_lemmas`.
const GRAMMATICAL_POS: [&str; 3] = ["pron", "card", "ind"];

/// Constructions of headwords, adverbs above all, that are another word's case forms.
const CASE_FORMED_ENDINGS: [&str; 7] = ["+ aṃ", "+ ena", "+ to", "+ e", "+ āya", "+ iṃ", "+ ssa"];

/// A verb form that is not the citation form: a present in -ti is, as bhavati is.
fn is_paradigm_form(term: &str, pos: &str) -> bool {
    FINITE_POS.contains(&pos) && !(pos == "pr" && term.ends_with("ti"))
}

/// How a headword's spelling matching a term should lean the term to it. A form that
/// is really an inflection gets no lean: a verb's other forms, and an adverb that is
/// another word's case form (dukkhaṁ from dukkha, vācāya from vācā).
fn citation_prior(term: &str, citation: &str, headword: &Headword) -> f64 {
    let adverbial_case = CASE_FORMED_ENDINGS
        .iter()
        .any(|ending| headword.construction.trim_end().ends_with(ending));
    if fold_niggahita(term) != citation || adverbial_case || is_paradigm_form(term, &headword.pos) {
        1.0
    } else if headword.pos == "ind" {
        INDECLINABLE_PRIOR
    } else {
        CITATION_PRIOR
    }
}

/// An aorist in -i, as a reading of a term that is not that aorist, when an a-stem word
/// beside it is another reading. DPD's forms of the aorist apāyi are the a-stem's own:
/// apāyaṁ is apāya, and counts cannot show it, because the two share every form.
fn is_shadowed_aorist(term: &str, headword: &Headword, nominals: &HashSet<String>) -> bool {
    let citation = fold_niggahita(bare_lemma(&headword.lemma));
    headword.pos == "aor"
        && fold_niggahita(term) != citation
        && citation.strip_suffix('i').is_some_and(|stem| nominals.contains(&format!("{stem}a")))
}

/// A causative, denominative or desiderative, as a reading of a future in -iss- that is
/// also a plain verb of the same root. Such a verb in -eti makes its future in -essati,
/// so gamissati and gamissāmi are gacchati's "will go", though DPD lists them under
/// gameti "makes go" too. Its optatives in -eyya stay its own: dasseyya "would show".
fn is_shadowed_voice(term: &str, headword: &Headword, plain_roots: &HashSet<&str>) -> bool {
    let citation = fold_niggahita(bare_lemma(&headword.lemma));
    let future = citation.strip_suffix("eti").is_some_and(|stem| term.starts_with(&format!("{stem}iss")));
    future && !separate_voice(headword).is_empty() && plain_roots.contains(headword.root_key.as_str())
}

/// The long- or short-vowel twin of a headword spelled exactly like the term, when the
/// two are different words: parinibbāyi is the aorist "attained Nibbāna", not
/// parinibbāyī "one who attains", and akampī "unshaken" is not akampi "shook".
/// Only -ī and -ū twins: -ā is also the plural of an a-stem, so cetiyā stays cetiya's.
fn is_shadowed_twin(term: &str, headword: &Headword, citations: &HashSet<String>, keys: &KeyFacts) -> bool {
    let term = fold_niggahita(term);
    let citation = fold_niggahita(bare_lemma(&headword.lemma));
    let long = if citation.ends_with(['ī', 'ū']) { &citation } else { &term };
    // Not -ā, which is also the plural of an a-stem: cetiyā is cetiya's.
    long.ends_with(['ī', 'ū'])
        && citation != term
        && citations.contains(&term)
        && short_final_vowel(&citation) == short_final_vowel(&term)
        && keys.unrelated_long.contains(long)
}

/// A term spelled like a noun in -ī or -ū is that noun, however common the word it
/// could also inflect: therī is not thera.
fn is_exact_noun(term: &str, citation: &str, headword: &Headword) -> bool {
    NOUN_POS.contains(&headword.pos.as_str()) && fold_niggahita(term) == citation && term.ends_with(['ī', 'ū'])
}

/// Each word's mass, by estimation over the terms and their readings.
fn estimate_masses(terms: &[(String, u64)], readings: &[Vec<Word>]) -> HashMap<String, f64> {
    let mut ids: HashMap<&str, usize> = HashMap::new();
    let compact: Vec<Vec<(usize, f64)>> = readings
        .iter()
        .map(|words| {
            words
                .iter()
                .map(|word| {
                    let next = ids.len();
                    (*ids.entry(word.key.as_str()).or_insert(next), word.prior)
                })
                .collect()
        })
        .collect();
    #[allow(clippy::cast_precision_loss)]
    let counts: Vec<f64> = terms.iter().map(|(_, count)| *count as f64).collect();
    let total: f64 = counts.iter().sum();

    let mut masses = vec![STARTING_MASS; ids.len()];
    for _ in 0..MAX_ESTIMATION_ROUNDS {
        let next = estimation_round(&counts, &compact, &masses);
        let change: f64 = next.iter().zip(&masses).map(|(new, old)| (new - old).abs()).sum();
        masses = next;
        if change <= SETTLED_SHARE * total {
            return ids.into_iter().map(|(key, id)| (key.to_string(), masses[id])).collect();
        }
    }
    eprintln!("word estimates had not settled after {MAX_ESTIMATION_ROUNDS} rounds");
    ids.into_iter().map(|(key, id)| (key.to_string(), masses[id])).collect()
}

fn estimation_round(counts: &[f64], readings: &[Vec<(usize, f64)>], masses: &[f64]) -> Vec<f64> {
    let mut next = vec![0.0; masses.len()];
    for (count, words) in counts.iter().zip(readings) {
        let weight = |&(id, prior): &(usize, f64)| weighted(masses[id], prior);
        let total: f64 = words.iter().map(weight).sum();
        for reading in words {
            next[reading.0] += count * weight(reading) / total;
        }
    }
    next
}

/// How much a reading counts, given the settled masses.
fn reading_weight(word: &Word, masses: &HashMap<String, f64>) -> f64 {
    weighted(masses.get(&word.key).copied().unwrap_or_default(), word.prior)
}

/// A mass can reach zero, which must not divide by zero.
fn weighted(mass: f64, prior: f64) -> f64 {
    mass.max(f64::MIN_POSITIVE) * prior
}

/// A term ending in sandhi -n or a lengthened vowel, as it would stand alone.
fn pre_sandhi_spelling(term: &str) -> Option<String> {
    let (last_at, last) = term.char_indices().last()?;
    let base = match last {
        'n' => 'ṁ',
        _ => short_vowel(last)?,
    };
    Some(format!("{}{base}", &term[..last_at]))
}

fn short_vowel(vowel: char) -> Option<char> {
    match vowel {
        'ā' => Some('a'),
        'ī' => Some('i'),
        'ū' => Some('u'),
        _ => None,
    }
}

/// DPD lists sati and satī as two lemmas of one word.
fn short_final_vowel(lemma: &str) -> String {
    let Some((at, last)) = lemma.char_indices().last() else {
        return String::new();
    };
    short_vowel(last).map_or_else(|| lemma.to_string(), |vowel| format!("{}{vowel}", &lemma[..at]))
}

/// A lemma's spelling for comparison, without numbers or a lengthened final vowel,
/// and without the consonant-stem ending of a word DPD also lists as -ā. A long vowel
/// stays where it tells words apart: saṅkhā is not saṅkha, nor nadī "river" nadi "roared".
fn lemma_spelling(lemma: &str, keys: &KeyFacts) -> String {
    let bare = fold_niggahita(bare_lemma(lemma));
    if keys.unrelated_long.contains(&bare) {
        return bare;
    }
    if keys.stem_twins.contains(&bare) {
        let stem = bare.strip_suffix("nt").or_else(|| bare.strip_suffix("r")).unwrap_or(&bare);
        return stem.to_string();
    }
    short_final_vowel(&bare)
}

/// DPD numbers senses and homonyms alike. Where one spelling has several roots, the
/// root tells the words apart: sati "mindfulness" is √sar, and sati "being" is √as.
/// A very short word keeps DPD's homonym number and exact spelling instead: ti "three"
/// is not ti "quotation", and mā "do not" is not ma "the letter". Any other headwords
/// spelled alike are one word.
fn word_key(headword: &Headword, keys: &KeyFacts) -> String {
    let spelling = lemma_spelling(&headword.lemma, keys);
    if keys.multi_root.contains(&spelling) && !headword.root_key.is_empty() {
        return format!("{spelling}|{}", headword.root_key);
    }
    if spelling.chars().count() < MIN_WORD_CHARS {
        let exact = fold_niggahita(bare_lemma(&headword.lemma));
        return homonym_number(&headword.lemma).map_or_else(|| exact.clone(), |number| format!("{exact} {number}"));
    }
    spelling
}

const MIN_WORD_CHARS: usize = 3;

/// Spellings so short (da, ma) that they are a poor reading of a longer term when a
/// real word is on offer. Pronouns and numerals stay: taṁ really is ta. A term also
/// keeps a headword spelled exactly like itself.
fn drop_tiny_lemmas(term: &str, words: Vec<Word>) -> Vec<Word> {
    let useful =
        |word: &Word| word.citation.chars().count() >= MIN_WORD_CHARS || word.grammatical || word.citation == term;
    if words.iter().any(useful) {
        words.into_iter().filter(useful).collect()
    } else {
        words
    }
}

/// Common readings counts cannot settle, because the forms are shared with a word
/// whose other forms are commoner: vutta is overwhelmingly "said", sati "mindfulness".
const PREFERRED_ROOTS: [(&str, &str); 14] = [
    ("vutta", "√vac"),
    ("sati", "√sar 2"),
    ("sutta", "√siv 1"),
    ("attha", "√atth"),
    ("kappa", "√kapp 1"),
    ("atthi", "√as 1"),
    ("suta", "√su 1"),
    ("sayaṁ", ""),
    ("kāya", "√ci 1"),
    ("āyati", "√yat 2"),
    ("asati", "√as 1"),
    ("asanta", "√as 1"),
    ("seyya", ""),
    ("dassati", "√dā 1"),
];

/// The 3 in "ka 3.1".
fn homonym_number(lemma: &str) -> Option<&str> {
    lemma.split(' ').nth(1)?.split('.').next()
}

const ENCLITICS: [&str; 10] = ["ca", "ce", "eva", "api", "pi", "iti", "ti", "hi", "va", "vā"];

/// The one word in a deconstruction such as "indriyaṃ + ca + eva" or "hi + etaṃ",
/// if everything else in it is an enclitic.
fn base_word(split: &str) -> Option<&str> {
    let mut content = split.split(" + ").filter(|part| !part.is_empty() && !ENCLITICS.contains(part));
    let base = content.next()?;
    content.next().is_none().then_some(base)
}

/// The content word of a sandhi headword such as "na + etaṃ". Negation counts as an
/// enclitic only here: nahetu, a word in its own right, is not hetu.
fn sandhi_content(construction: &str) -> Option<&str> {
    base_word(construction.strip_prefix("na + ").unwrap_or(construction))
}

const VERBAL_POS: [&str; 14] = [
    "pr", "aor", "fut", "opt", "imp", "cond", "perf", "abs", "ger", "inf", "pp", "prp", "ptp", "imperf",
];
const SEPARATE_VOICES: [&str; 3] = ["caus", "deno", "desid"];

/// An index term shared by the verbal forms of a root, so that gacchati finds gata.
/// A negated form stays out: appaṭivirata is the opposite of paṭivirata. Causatives,
/// denominatives and desideratives are separate verbs, with families of their own.
fn verb_family(headword: &Headword) -> Option<String> {
    let verbal = VERBAL_POS.contains(&headword.pos.as_str());
    let negated = headword.construction.starts_with("na ");
    if !verbal || negated || headword.root_key.is_empty() {
        return None;
    }
    Some(format!("family:{}|{}|{}", headword.family_root, headword.root_key, separate_voice(headword)))
}

/// "caus", "deno" or "desid" for a verb DPD derives that way, and "" otherwise.
fn separate_voice(headword: &Headword) -> &'static str {
    SEPARATE_VOICES
        .into_iter()
        .find(|voice| headword.root_base.contains(voice))
        .unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(clippy::too_many_lines)]
    fn dictionary() -> Dictionary {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            r#"CREATE TABLE dpd_headwords (id INTEGER, lemma_1 TEXT, pos TEXT, root_key TEXT,
                family_root TEXT, root_base TEXT, construction TEXT);
            CREATE TABLE lookup (lookup_key TEXT, headwords TEXT, deconstructor TEXT);
            INSERT INTO dpd_headwords VALUES
                (1, 'paññā 1', 'fem', '√ñā 1', 'pa √ñā', '', ''),
                (2, 'paññāyati', 'pr', '√ñā 1', 'pa √ñā', '', ''),
                (3, 'gacchati 1', 'pr', '√gam 1', '√gam', '', '√gam + a + ti'),
                (4, 'gata 1', 'pp', '√gam 1', '√gam', '', '√gam + ta'),
                (5, 'gata 4', 'nt', '√gam 1', '√gam', '', '√gam + ta'),
                (6, 'agata', 'pp', '√gam 1', '√gam', '', 'na > a + √gam + ta'),
                (7, 'sati 1.1', 'fem', '√sar 2', '√sar', '', ''),
                (8, 'sati 2.1', 'loc', '√as 1', '√as', '', ''),
                (27, 'bhagavant', 'masc', '', '', '', ''),
                (28, 'bhagavā', 'masc', '', '', '', ''),
                (29, 'dukkhaṃ', 'ind', '', '', '', 'dukkha + aṃ'),
                (30, 'netaṃ', 'sandhi', '', '', '', 'na + etaṃ'),
                (31, 'neti', 'pr', '√nī', '√nī', '', ''),
                (32, 'gamissati', 'fut', '√gam 1', '√gam', '', ''),
                (33, 'pāpa', 'nt', '', '', '', ''),
                (34, 'pāpa 2', 'adj', '', '', '', ''),
                (35, 'pāpā', 'imperf', '√pat', 'pa √pat', '', ''),
                (36, 'ye 1', 'pron', '', '', '', ''),
                (37, 'yāti', 'pr', '√yā', '√yā', '', ''),
                (38, 'ya', 'pron', '', '', '', ''),
                (39, 'ta', 'pron', '', '', '', ''),
                (40, 'tvaṃ', 'pron', '', '', '', ''),
                (41, 'no 1.1', 'ind', '', '', '', 'na + u > no'),
                (42, 'ahaṃ', 'pron', '', '', '', ''),
                (43, 'bhava', 'masc', '√bhū', '√bhū', '', ''),
                (44, 'bhavant', 'pron', '√bhū', '√bhū', '', ''),
                (45, 'kathā', 'fem', '', '', '', ''),
                (46, 'kathaṃ', 'ind', '', '', '', 'ka + thaṃ'),
                (47, 'hetu', 'masc', '', '', '', ''),
                (48, 'santi 1.1', 'fem', '√sam', '√sam', '', ''),
                (49, 'santi', 'sandhi', '', '', '', 'saṃ + iti'),
                (50, 'saṃ', 'ind', '', '', '', ''),
                (51, 'vadāmi', 'pr', '√vad', '√vad', '', 'vada + āmi'),
                (52, 'vadati', 'pr', '√vad', '√vad', '', ''),
                (53, 'sacca', 'nt', '', '', '', ''),
                (54, 'saccaṃ', 'ind', '', '', '', 'sacca + aṃ'),
                (55, 'tesaṃ', 'pron', '', '', '', ''),
                (56, 'bhavati', 'pr', '√bhū', '√bhū', '', ''),
                (57, 'bhonta', 'prp', '√bhū', '√bhū', '', ''),
                (58, 'ca 1.2', 'letter', '', '', '', ''),
                (59, 'ca 1.1', 'ind', '', '', '', ''),
                (60, 'ce', 'ind', '', '', '', ''),
                (61, 'vācā', 'fem', '', '', '', ''),
                (62, 'vācāya', 'ind', '', '', '', 'vācā + āya'),
                (63, 'satthā', 'masc', '', '', '', ''),
                (64, 'satthar', 'masc', '', '', '', ''),
                (65, 'buddha 1', 'masc', '√budh', '√budh', '', ''),
                (66, 'buddha 2', 'pp', '√budh', '√budh', '', ''),
                (67, 'sandhione', 'sandhi', '', '', '', 'na + etaṃ'),
                (68, 'sandhitwo', 'sandhi', '', '', '', ''),
                (69, 'nobase', 'sandhi', '', '', '', 'missing + ca'),
                (70, 'realword', 'masc', '', '', '', ''),
                (71, 'apāya 1', 'masc', '√i 1', 'apa √i', '', ''),
                (72, 'apāyi 2.1', 'aor', '√i 1', 'apa √i', '', ''),
                (73, 'thera 1.1', 'adj', '√ṭhā', '√ṭhā', '', ''),
                (74, 'therī', 'fem', '√ṭhā', '√ṭhā', '', ''),
                (75, 'saṅkhā 1', 'fem', '√khā', 'saṃ √khā', '', ''),
                (76, 'saṅkha 1', 'masc', '', '', '', ''),
                (77, 'sampaka', 'pp', '√pac', '√pac', '', ''),
                (78, 'sampakata', 'nt', '', '', '', ''),
                (83, 'parinibbāyi', 'aor', '√vā', 'pari nī √vā', '', ''),
                (84, 'parinibbāyī', 'adj', '√vā', 'pari nī √vā', '', ''),
                (85, 'parinibbāyati', 'pr', '√vā', 'pari nī √vā', '', ''),
                (80, 'nadī', 'fem', '', '', '', ''),
                (81, 'nadi', 'aor', '√nad', '√nad', '', ''),
                (82, 'nadati', 'pr', '√nad', '√nad', '', ''),
                (79, 'gameti 1', 'pr', '√gam 1', '√gam', '√gam + *e > game (caus, irreg)', ''),
                (9, 'kāreti', 'pr', '√kar', '√kar', '√kar + *e > kāre (caus, irreg)', ''),
                (10, 'karoti', 'pr', '√kar', '√kar', '', ''),
                (11, 'kata', 'pp', '√kar', '√kar', '', ''),
                (12, 'evaṃ', 'ind', '', '', '', ''),
                (13, 'hetaṃ', 'sandhi', '', '', '', ''),
                (14, 'rāja 1', 'masc', '', '', '', ''),
                (15, 'etaṃ', 'pron', '', '', '', ''),
                (16, 'puna', 'ind', '', '', '', ''),
                (17, 'punāti', 'pr', '√pu', '√pu', '', ''),
                (18, 'vutta 1', 'pp', '√vap', '√vap', '', ''),
                (19, 'vutta 2', 'pp', '√vac', '√vac', '', ''),
                (20, 'vuccati', 'pr', '√vac', '√vac', '', ''),
                (21, 'ti 1', 'card', '', '', '', ''),
                (22, 'ti 2', 'ind', '', '', '', ''),
                (23, 'sutta 1', 'pp', '√sup', '√sup', '', ''),
                (24, 'sutta 2', 'nt', '√siv 1', '√siv', '', ''),
                (25, 'satī', 'fem', '√sar 2', '√sar', '', ''),
                (26, 'dukkha', 'nt', '', '', '', '');
            INSERT INTO lookup VALUES ('paññāya', '[2, 1]', ''), ('paññā', '[1]', ''),
                ('paññāyati', '[2]', ''), ('gacchati', '[3]', ''), ('gata', '[4, 5]', ''),
                ('agata', '[6]', ''), ('sati', '[7, 8]', ''), ('kāreti', '[9]', ''),
                ('karoti', '[10]', ''), ('kataṃ', '[11]', ''), ('evaṃ', '[12]', ''),
                ('rañño', '[14]', ''), ('rājā', '[14]', ''), ('etaṃ', '[15]', ''),
                ('hetaṃ', '[13]', '["hi + etaṃ"]'), ('satiyā', '[25]', ''),
                ('bhagavā', '[28]', ''), ('bhagavato', '[27]', ''), ('dukkhassa', '[26]', ''),
                ('dukkhaṃ', '[29, 26]', ''), ('netaṃ', '[31, 30]', ''),
                ('gamissati', '[32, 79, 3]', ''), ('gacchanti', '[3]', ''), ('gameti', '[79]', ''),
                ('gamessati', '[79]', ''), ('gamissāmi', '[79, 32]', ''), ('gameyya', '[79, 32]', ''), ('nadī', '[80, 81]', ''), ('nadiyā', '[80]', ''), ('nadi', '[81, 80]', ''), ('nadanti', '[82]', ''), ('parinibbāyi', '[85, 83, 84]', ''), ('parinibbāyino', '[84]', ''), ('parinibbāyī', '[83, 84]', ''), ('parinibbāyanti', '[85]', ''), ('pāpaṃ', '[33, 34]', ''), ('taṃ', '[40, 39]', ''), ('tassa', '[39]', ''),
                ('tañca', '', '["taṃ + ca"]'), ('no', '[42, 41]', ''), ('mayaṃ', '[42]', ''),
                ('bho', '[44]', ''), ('bhavo', '[43]', ''), ('kathaṃ', '[46, 45]', ''), ('kathāya', '[45]', ''),
                ('kathañca', '', '["kathaṃ + ca"]'), ('nahetu', '', '["na + hetu"]'), ('hetu', '[47]', ''),
                ('santi', '[48, 49]', ''), ('vadāmi', '[51, 52]', ''), ('vadanti', '[52]', ''),
                ('saccaṃ', '[54, 53]', ''), ('tesaṃ', '[55, 39]', ''), ('bhavati', '[56, 57]', ''),
                ('bhonto', '[57]', ''), ('ceva', '[58, 59, 60]', ''), ('vācāya', '[62, 61]', ''), ('vācā', '[61]', ''),
                ('satthu', '[64]', ''), ('satthā', '[63]', ''), ('buddho', '[65, 66]', ''),
                ('twosandhi', '[67, 68]', ''), ('realform', '[69, 70]', ''), ('saccassa', '[53]', ''), ('ye', '[37, 36, 38]', ''),
                ('puna', '[17, 16]', ''), ('punanti', '[17]', ''), ('vuttaṃ', '[18, 19]', ''),
                                ('vuccati', '[20]', ''), ('ti', '[21, 22]', ''), ('suttaṃ', '[23, 24]', ''),
                ('kaputta', '', '["ka + putta"]'),
                ('sampakaṃ', '[77, 78]', ''), ('saṃ', '[50]', ''),
                ('apāyaṃ', '[71, 72]', ''), ('apāyi', '[71, 72]', ''),
                ('therī', '[73, 74]', ''), ('thero', '[73]', ''),
                ('saṅkhā', '[75, 76]', ''), ('saṅkhāya', '[75]', ''), ('saṅkho', '[76]', ''),
                ('evaṃceva', '', '["evaṃ + ca + eva"]');"#,
        )
        .unwrap();
        Dictionary::from(conn)
    }

    fn entries(terms: &[(&str, u64)]) -> HashMap<String, TermEntry> {
        let terms: Vec<(String, u64)> = terms.iter().map(|(t, c)| ((*t).to_string(), *c)).collect();
        dictionary().entries(&terms).unwrap()
    }

    fn word(term: &str, others: &[(&str, u64)]) -> Option<String> {
        let mut terms = vec![(term, 1)];
        terms.extend_from_slice(others);
        entries(&terms)[term].word.clone()
    }

    fn family(term: &str) -> Option<String> {
        entries(&[(term, 1)])[term].family.clone()
    }

    fn verb_family_of(root_key: &str) -> Option<String> {
        verb_family(&Headword {
            lemma: String::from("x"),
            pos: String::from("pr"),
            root_key: root_key.to_string(),
            family_root: String::from("√vid"),
            root_base: String::new(),
            construction: String::new(),
        })
    }

    #[test]
    fn test_an_aorist_does_not_take_the_forms_of_its_a_stem_twin() {
        assert_eq!(word("apāyaṁ", &[]).as_deref(), Some("apāya"));
        assert!(entries(&[("apāyaṁ", 1)])["apāyaṁ"].family.is_none());
    }

    #[test]
    fn test_an_aorist_keeps_its_own_spelling() {
        let aorist = Headword {
            lemma: String::from("apāyi 2.1"),
            pos: String::from("aor"),
            root_key: String::new(),
            family_root: String::new(),
            root_base: String::new(),
            construction: String::new(),
        };
        let nominals = HashSet::from([String::from("apāya")]);
        assert!(!is_shadowed_aorist("apāyi", &aorist, &nominals));
        assert!(is_shadowed_aorist("apāyaṁ", &aorist, &nominals));
        assert!(!is_shadowed_aorist("apāyaṁ", &aorist, &HashSet::new()));
    }

    #[test]
    fn test_a_noun_in_long_i_outranks_a_commoner_word() {
        assert_eq!(word("therī", &[("thero", 5000)]).as_deref(), Some("theri"));
    }

    #[test]
    fn test_a_final_long_vowel_with_no_shared_root_is_another_word() {
        assert_ne!(word("saṅkhāya", &[]), word("saṅkho", &[]));
        assert_eq!(word("saṅkhā", &[("saṅkhāya", 5)]), word("saṅkhāya", &[]));
    }

    #[test]
    fn test_preferred_roots_are_checked_against_the_dictionary() {
        let error = dictionary().check_preferred_roots().unwrap_err().to_string();
        assert!(error.contains("PREFERRED_ROOTS"), "{error}");
    }

    #[test]
    fn test_a_rare_word_loses_its_shared_forms() {
        let frequent = [("dukkhassa", 100)];
        assert_eq!(word("dukkhaṁ", &frequent).as_deref(), Some("dukkha"));
    }

    #[test]
    fn test_a_form_absent_from_the_texts_still_gets_a_word() {
        assert_eq!(entries(&[("paññā", 0), ("paññāya", 0)])["paññāya"].word.as_deref(), Some("pañña"));
    }

    #[test]
    fn test_finite_forms_listed_as_headwords_join_their_verb() {
        let frequent = [("gacchanti", 100)];
        assert_eq!(word("gamissati", &frequent), word("gacchanti", &frequent));
    }

    #[test]
    fn test_a_causative_does_not_take_a_plain_verbs_own_headword() {
        let frequent = [("gameti", 500), ("gamessati", 500)];
        assert_ne!(word("gamissati", &frequent), word("gameti", &frequent));
        assert_eq!(entries(&[("gamissati", 1)])["gamissati"].family, family("gacchati"));
        assert_eq!(word("gamissāmi", &frequent), word("gamissati", &frequent));
        assert_eq!(word("gamessati", &[]), word("gameti", &[]));
        assert_eq!(word("gameyya", &frequent), word("gameti", &frequent));
    }

    #[test]
    fn test_a_noun_in_long_i_is_not_the_aorist_spelled_short() {
        let frequent = [("nadanti", 50)];
        assert_eq!(word("nadī", &frequent), word("nadiyā", &frequent));
        assert_eq!(word("nadiyā", &frequent).as_deref(), Some("nadī"));
        assert_eq!(entries(&[("nadiyā", 1)])["nadiyā"].family, None);
    }

    #[test]
    fn test_an_aorist_keeps_its_own_spelling_beside_a_long_vowel_noun() {
        let frequent = [("parinibbāyino", 50), ("parinibbāyanti", 5)];
        let terms = entries(&[("parinibbāyi", 10), ("parinibbāyī", 10), frequent[0], frequent[1]]);
        assert_eq!(terms["parinibbāyi"].family, terms["parinibbāyanti"].family);
        assert!(terms["parinibbāyi"].family.is_some());
        assert_eq!(terms["parinibbāyī"].word, terms["parinibbāyino"].word);
        assert_eq!(terms["parinibbāyī"].family, None);
        assert_ne!(terms["parinibbāyī"].word, terms["parinibbāyi"].word);
    }

    #[test]
    fn test_a_sandhi_construction_outranks_a_look_alike() {
        assert_eq!(word("netaṁ", &[("neti", 500)]), word("etaṁ", &[]));
    }

    #[test]
    fn test_variant_stems_are_one_word() {
        assert_eq!(word("bhagavā", &[]), word("bhagavato", &[]));
    }

    #[test]
    fn test_a_term_keeps_a_short_headword_spelled_like_itself() {
        assert_ne!(word("ye", &[]).as_deref(), Some("yāti"));
    }

    #[test]
    fn test_a_rare_verbal_homograph_gives_a_word_no_family() {
        assert_eq!(family("pāpaṁ"), None);
    }

    #[test]
    fn test_estimation_changes_its_mind_over_rounds() {
        let frequent = [("tesaṁ", 100), ("tassa", 120)];
        assert_eq!(entries(&frequent)["tesaṁ"].word, entries(&frequent)["tassa"].word);
    }

    #[test]
    fn test_a_present_in_ti_keeps_its_citation_lean() {
        let frequent = [("bhavati", 40), ("bhonto", 20)];
        assert_eq!(entries(&frequent)["bhavati"].word.as_deref(), Some("bhavati"));
    }

    #[test]
    fn test_a_word_is_grammatical_if_any_headword_is() {
        assert_ne!(word("ceva", &[("ceva", 1)]).as_deref(), Some("ce"));
    }

    #[test]
    fn test_a_case_formed_adverb_stays_with_its_noun() {
        let frequent = [("vācā", 5)];
        assert_eq!(word("vācāya", &frequent), word("vācā", &frequent));
    }

    #[test]
    fn test_ar_stems_join_their_long_vowel_twin() {
        assert_eq!(word("satthu", &[]), word("satthā", &[]));
    }

    #[test]
    fn test_an_even_split_goes_by_the_first_headword() {
        assert_eq!(family("buddho"), None);
    }

    #[test]
    fn test_the_first_sandhi_analysis_wins() {
        assert_eq!(word("twosandhi", &[]), word("etaṁ", &[]));
    }

    #[test]
    fn test_a_sandhi_base_dpd_cannot_find_leaves_the_real_reading() {
        assert_eq!(word("realform", &[]).as_deref(), Some("realword"));
    }

    #[test]
    fn test_a_short_pronoun_keeps_its_forms() {
        let frequent = [("tassa", 500)];
        assert_eq!(word("taṁ", &frequent), word("tassa", &frequent));
    }

    #[test]
    fn test_a_base_from_a_deconstruction_keeps_its_lean() {
        assert_eq!(word("kathañca", &[("kathaṁ", 10), ("kathāya", 50)]).as_deref(), Some("kathaṁ"));
    }

    #[test]
    fn test_an_indeclinable_with_a_construction_keeps_its_lean() {
        let terms = entries(&[("no", 100), ("mayaṁ", 50)]);
        assert_eq!(terms["no"].word.as_deref(), Some("no 1"));
    }

    #[test]
    fn test_an_adverbial_accusative_stays_with_its_noun() {
        let frequent = [("saccassa", 5)];
        assert_eq!(word("saccaṁ", &frequent), word("saccassa", &frequent));
    }

    #[test]
    fn test_nt_is_kept_without_a_long_vowel_twin() {
        assert_ne!(word("bho", &[]), word("bhavo", &[]));
    }

    #[test]
    fn test_a_negated_compound_is_not_its_positive_word() {
        assert_eq!(word("nahetu", &[]), None);
    }

    #[test]
    fn test_a_real_headword_outranks_a_sandhi_analysis() {
        assert_eq!(word("santi", &[]).as_deref(), Some("santi"));
    }

    #[test]
    fn test_present_forms_listed_as_headwords_join_their_verb() {
        let frequent = [("vadanti", 50)];
        assert_eq!(word("vadāmi", &frequent), word("vadanti", &frequent));
    }

    #[test]
    fn test_finds_a_term_spelled_with_the_other_niggahita() {
        assert_eq!(word("evaṁ", &[]).as_deref(), Some("evaṁ"));
    }

    #[test]
    fn test_the_commoner_word_takes_an_ambiguous_term() {
        let frequent = [("paññā", 500), ("paññāyati", 5)];
        assert_eq!(word("paññāya", &frequent), word("paññā", &frequent));
    }

    #[test]
    fn test_a_term_that_is_a_headword_reads_as_that_headword() {
        assert_eq!(word("puna", &[("punanti", 1)]).as_deref(), Some("puna"));
    }

    #[test]
    fn test_a_preferred_root_settles_what_counts_cannot() {
        assert_eq!(word("vuttaṁ", &[]).as_deref(), Some("vutta|√vac"));
    }

    #[test]
    fn test_a_tie_goes_to_the_noun() {
        assert_eq!(word("sampakaṁ", &[]).as_deref(), Some("sampakata"));
    }

    #[test]
    fn test_rootless_homonyms_stay_apart() {
        assert_eq!(word("ti", &[]).as_deref(), Some("ti 2"));
    }

    #[test]
    fn test_headwords_with_one_root_are_one_word() {
        assert_eq!(word("kataṁ", &[]).as_deref(), Some("kata"));
    }

    #[test]
    fn test_variant_final_vowels_are_one_word() {
        let satī = Headword {
            lemma: String::from("satī"),
            pos: String::from("fem"),
            root_key: String::from("√sar 1"),
            family_root: String::new(),
            root_base: String::new(),
            construction: String::new(),
        };
        let multi_root = HashSet::from([String::from("sati")]);
        let keys = KeyFacts {
            multi_root,
            stem_twins: HashSet::new(),
            unrelated_long: HashSet::new(),
            families: HashMap::new(),
            headwords: HashMap::new(),
        };
        assert_eq!(word_key(&satī, &keys), "sati|√sar 1");
    }

    #[test]
    fn test_sandhi_spelling_finds_its_word() {
        assert_eq!(word("dukkhan", &[]), word("dukkhaṁ", &[]));
        assert_eq!(pre_sandhi_spelling("bhavissatī").as_deref(), Some("bhavissati"));
        assert_eq!(pre_sandhi_spelling("dhammo"), None);
        assert_eq!(pre_sandhi_spelling(""), None);
    }

    #[test]
    fn test_forms_of_one_word_share_a_term_whatever_their_spelling() {
        assert_eq!(word("rañño", &[]), word("rājā", &[]));
    }

    #[test]
    fn test_the_root_tells_homonyms_apart() {
        assert_eq!(word("sati", &[("satiyā", 5)]).as_deref(), Some("sati|√sar 2"));
    }

    #[test]
    fn test_word_plus_enclitics_takes_that_words_entry() {
        assert_eq!(word("evaṁceva", &[]), word("evaṁ", &[]));
    }

    #[test]
    fn test_sandhi_headword_goes_to_its_content_word() {
        assert_eq!(word("hetaṁ", &[]), word("etaṁ", &[]));
    }

    #[test]
    fn test_unknown_terms_and_compounds_have_no_word() {
        assert_eq!(word("xyz", &[]), None);
        assert_eq!(word("kaputta", &[]), None);
    }

    #[test]
    fn test_verbal_forms_of_a_root_share_a_family() {
        assert!(family("gacchati").is_some());
        assert_eq!(family("gacchati"), family("gata"));
    }

    #[test]
    fn test_prefixless_forms_of_a_root_share_a_family() {
        assert_eq!(family("karoti"), family("kataṁ"));
    }

    #[test]
    fn test_numbered_roots_are_different_roots() {
        assert_ne!(verb_family_of("√vid 1"), verb_family_of("√vid 6"));
    }

    #[test]
    fn test_negated_forms_and_nouns_have_no_family() {
        assert_eq!(family("agata"), None);
        assert_eq!(family("paññā"), None);
    }

    #[test]
    fn test_a_causative_is_a_family_of_its_own_whatever_its_label() {
        assert!(family("kāreti").is_some());
        assert_ne!(family("kāreti"), family("karoti"));
    }

    #[test]
    fn test_lemmas_lose_their_numbers_and_use_the_tokenizer_niggahita() {
        let lemmas = dictionary().lemmas().unwrap();
        assert!(lemmas.contains(&String::from("evaṁ")));
        assert!(lemmas.contains(&String::from("sati")));
    }
}
