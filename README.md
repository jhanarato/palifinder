# A search engine for Pāli

There is currently no language analyzer for the Pāli language for any of the major search engine servers and frameworks. This project seeks to rectify that.

The main objectives:

- [X] Extract the vocabulary of the Pāli tipitika using the data from SuttaCentral.
- [X] Find the stem for each word using the Digital Pāli Dictionary. (50% coverage so far)
- [X] Create a dictionary stemmer.
- [X] Create an algorithmic stemmer in Snowball.
- [ ] Test the performance of the algorithmic stemmer against the "perfect" dictionary stemmer.
- [X] Investigate RAM usage of dictionary stemmer. (Probably around 5MB)
- [ ] Integrate the stemmers with Tantivy.
- [ ] Index the tipitika with Tantivy.
- [ ] Create a command line interface to search the tipitika.

# Instructions

Download the Digital Pāli Dictionary here:

https://github.com/digitalpalidictionary/dpd-db/releases/download/v0.4.20260728/dpd.db.tar.xz

Extract the file into `data/`.