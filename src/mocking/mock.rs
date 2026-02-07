use std::str::FromStr;

use apca::data::v2::stream::{Bar, Data, Quote, Trade};

use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use csv::ReaderBuilder;
use polars::{
    frame::DataFrame,
    io::SerReader,
    prelude::{col, CsvReadOptions, DataType, IntoLazy, TimeUnit},
};
use serde::Deserialize;
use tokio_stream::{Stream, StreamExt};

use crate::error::CLIError;

/// CSV record structure matching the OHLCV format
#[derive(Debug, Deserialize)]
struct CsvRecord {
    #[serde(rename = "Date")]
    date: String,
    #[serde(rename = "Open")]
    open: f64,
    #[serde(rename = "High")]
    high: f64,
    #[serde(rename = "Low")]
    low: f64,
    #[serde(rename = "Close")]
    close: f64,
    #[serde(rename = "Adj Close")]
    adj_close: f64,
    #[serde(rename = "Volume")]
    volume: f64,
}

/// Converts OHLC values to a Bar object
pub fn values_to_bar(
    symbol: &str,
    timestamp: DateTime<Utc>,
    open: f64,
    close: f64,
    high: f64,
    low: f64,
    volume: f64,
) -> Bar {
    use num_decimal::Num;
    Bar {
        symbol: symbol.to_string(),
        open_price: Num::from_str(&open.to_string()).unwrap(),
        high_price: Num::from_str(&high.to_string()).unwrap(),
        low_price: Num::from_str(&low.to_string()).unwrap(),
        close_price: Num::from_str(&close.to_string()).unwrap(),
        volume: Num::from_str(&volume.to_string()).unwrap(),
        timestamp,
    }
}

//Data<Bar, Quote, Trade>
pub fn data_csv(filename: String) -> Result<DataFrame, CLIError> {
    let df = CsvReadOptions::default()
        .map_parse_options(|parse_options| parse_options.with_try_parse_dates(true))
        .try_into_reader_with_file_path(Some(filename.into()))
        .unwrap()
        .finish()?;
    let df = df
        .lazy()
        .with_column(col("Date").cast(DataType::Datetime(TimeUnit::Milliseconds, None)))
        .collect()?;

    Ok(df)
}

/// Creates a stream of Data<Bar, Quote, Trade> from a CSV file
/// This is a cleaner version that reads CSV directly without complex polars operations
///
/// # Arguments
/// * `filename` - Path to the CSV file
/// * `symbol` - Stock symbol to use for the bars
///
/// # Returns
/// A stream of Data<Bar, Quote, Trade>
pub async fn data_stream_from_csv(
    filename: &str,
    symbol: &str,
) -> Result<
    impl Stream<Item = Result<Data<Bar, Quote, Trade>, Box<dyn std::error::Error + Send + Sync>>>,
    Box<dyn std::error::Error + Send + Sync>,
> {
    let mut reader = ReaderBuilder::new().has_headers(true).from_path(filename)?;

    let symbol = symbol.to_string();
    let mut bars = Vec::new();

    // Simple iteration - no zip chains!
    for result in reader.deserialize() {
        let record: CsvRecord = result?;

        // Parse the date string to DateTime<Utc>
        let naive_date = NaiveDateTime::parse_from_str(
            &format!("{} 00:00:00", record.date),
            "%Y-%m-%d %H:%M:%S",
        )?;
        let timestamp = DateTime::<Utc>::from_naive_utc_and_offset(naive_date, Utc);

        let bar = values_to_bar(
            &symbol,
            timestamp,
            record.open,
            record.close,
            record.high,
            record.low,
            record.volume,
        );
        bars.push(Data::Bar(bar));
    }

    // Create a stream from the vector
    let stream = tokio_stream::iter(bars).map(Ok);
    Ok(stream)
}
/// Creates a mock data stream from a CSV file
///
/// # Arguments
/// * `filename` - Path to the CSV file (e.g., "files/orcl.csv")
/// * `symbol` - Stock symbol to use for the bars (e.g., "ORCL")
/// * `delay_ms` - Delay in milliseconds between each data point
///
/// # Returns
/// A stream of Result<Ok<Data<Bar, Quote, Trade>>, serde_json::Error>
///
/// # Example
/// ```no_run
/// use crate::mocking::mock::mock_data_stream;
///
/// let stream = mock_data_stream("files/orcl.csv", "ORCL", 1).await?;
/// ```
pub async fn mock_data_stream(
    filename: &str,
    symbol: &str,
    delay_ms: u64,
) -> Result<
    impl Stream<Item = Result<Data<Bar, Quote, Trade>, Box<dyn std::error::Error + Send + Sync>>>,
    Box<dyn std::error::Error + Send + Sync>,
> {
    let mut reader = ReaderBuilder::new().has_headers(true).from_path(filename)?;

    let symbol = symbol.to_string();
    let mut bars = Vec::new();

    for result in reader.deserialize() {
        let record: CsvRecord = result?;

        // Parse the date string to DateTime<Utc>
        // Try parsing as YYYY-MM-DD format
        let naive_date = NaiveDateTime::parse_from_str(
            &format!("{} 00:00:00", record.date),
            "%Y-%m-%d %H:%M:%S",
        )?;
        let timestamp = DateTime::<Utc>::from_naive_utc_and_offset(naive_date, Utc);

        let bar = values_to_bar(
            &symbol,
            timestamp,
            record.open,
            record.close,
            record.high,
            record.low,
            record.volume,
        );
        bars.push(Data::Bar(bar));
    }

    // Create a stream from the vector with delays
    let stream = tokio_stream::iter(bars).then(move |data| async move {
        if delay_ms > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
        }
        Ok(data)
    });

    Ok(stream)
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_mock_data_stream() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Create a temporary CSV file
        let mut file = NamedTempFile::new()?;
        writeln!(file, "Date,Open,High,Low,Close,Adj Close,Volume")?;
        writeln!(file, "2023-01-01,100.0,105.0,95.0,102.0,102.0,1000")?;

        let path = file.path().to_str().unwrap();
        let stream = mock_data_stream(path, "TEST", 0).await?;
        tokio::pin!(stream);

        // Get the first item from the stream
        if let Some(result) = stream.next().await {
            let data = result.unwrap();
            match data {
                Data::Bar(bar) => {
                    assert_eq!(bar.symbol, "TEST");
                    assert_eq!(bar.open_price.to_string(), "100");
                    assert_eq!(bar.close_price.to_string(), "102");
                    assert_eq!(bar.high_price.to_string(), "105");
                    assert_eq!(bar.low_price.to_string(), "95");
                }
                _ => panic!("Expected Data::Bar"),
            }
        } else {
            panic!("Stream was empty");
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_data_stream_from_csv() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Read from actual orcl.csv file
        let path = "files/orcl.csv";
        let stream = data_stream_from_csv(path, "ORCL").await?;
        tokio::pin!(stream);

        // Get the first item from the stream
        loop {
            if let Some(result) = stream.next().await {
                let data = result.unwrap();
                println!("First bar from orcl.csv: {:#?}", data);
                match data {
                    Data::Bar(bar) => {
                        assert_eq!(bar.symbol, "ORCL");
                        // Just verify we got a bar with some data
                        assert!(bar.open_price.to_string().parse::<f64>().unwrap() > 0.0);
                        assert!(bar.close_price.to_string().parse::<f64>().unwrap() > 0.0);
                    }
                    _ => panic!("Expected Data::Bar"),
                }
            } else {
                panic!("Stream was empty");
            }
            break;
        }
        panic!("Stream was empty");
        Ok(())
    }
}
