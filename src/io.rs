use super::types::{Transaction, TransactionFields};

pub struct CsvFileReader {
    record_iter: csv::DeserializeRecordsIntoIter<std::fs::File, TransactionFields>,
}

impl CsvFileReader {
    pub fn new(input_filename: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            record_iter: csv::ReaderBuilder::new()
                .has_headers(true)
                .trim(csv::Trim::All)
                .from_path(input_filename)?
                .into_deserialize(),
        })
    }
}

impl Iterator for CsvFileReader {
    type Item = Result<Transaction, Box<dyn std::error::Error>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.record_iter.next().map(|result| match result {
            Ok(fields) => Transaction::try_from(fields).map_err(|e| e.into()),
            Err(e) => Err(e.into()),
        })
    }
}
