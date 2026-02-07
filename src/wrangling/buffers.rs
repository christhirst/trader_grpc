use apca::data::v2::stream::{Bar, Trade};

#[derive(Debug)]
pub struct Buffer {
    symbol: String,
    size: usize,
    trades: Option<Vec<Trade>>,
    bars: Option<Vec<Bar>>,
}

impl Buffer {
    /// Create a new buffer for a specific symbol with a maximum size
    pub fn new(symbol: String, size: usize) -> Self {
        Self {
            symbol,
            size,
            trades: Some(Vec::new()),
            bars: Some(Vec::new()),
        }
    }

    /// Add a trade to the buffer, removing the oldest if size limit is reached
    pub fn add_trade(&mut self, trade: Trade) {
        if let Some(ref mut trades) = self.trades {
            if trades.len() >= self.size {
                trades.remove(0);
            }
            trades.push(trade);
        }
    }

    /// Add a bar to the buffer, removing the oldest if size limit is reached
    pub fn add_bar(&mut self, bar: Bar) {
        if let Some(ref mut bars) = self.bars {
            if bars.len() >= self.size {
                bars.remove(0);
            }
            bars.push(bar);
        }
    }

    /// Get a reference to the trades
    pub fn get_trades(&self) -> Option<&Vec<Trade>> {
        self.trades.as_ref()
    }

    /// Get a reference to the bars
    pub fn get_bars(&self) -> Option<&Vec<Bar>> {
        self.bars.as_ref()
    }

    /// Get the symbol
    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    /// Get the current number of trades stored
    pub fn trade_count(&self) -> usize {
        self.trades.as_ref().map(|t| t.len()).unwrap_or(0)
    }

    /// Get the current number of bars stored
    pub fn bar_count(&self) -> usize {
        self.bars.as_ref().map(|b| b.len()).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mocking::mock::values_to_bar;
    use chrono::Utc;

    #[tokio::test]
    async fn test_buffer_initialization() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let buffer = Buffer::new("AAPL".to_string(), 10);

        assert_eq!(buffer.symbol(), "AAPL");
        assert_eq!(buffer.trade_count(), 0);
        assert_eq!(buffer.bar_count(), 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_add_single_bar() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut buffer = Buffer::new("AAPL".to_string(), 10);
        let bar = values_to_bar("AAPL", Utc::now(), 100.0, 105.0, 110.0, 95.0, 1000.0);

        buffer.add_bar(bar);

        assert_eq!(buffer.bar_count(), 1);
        assert_eq!(buffer.trade_count(), 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_add_multiple_bars() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut buffer = Buffer::new("AAPL".to_string(), 10);

        for i in 0..5 {
            let bar = values_to_bar(
                "AAPL",
                Utc::now(),
                100.0 + i as f64,
                105.0 + i as f64,
                110.0 + i as f64,
                95.0 + i as f64,
                1000.0,
            );
            buffer.add_bar(bar);
        }

        assert_eq!(buffer.bar_count(), 5);

        Ok(())
    }

    #[tokio::test]
    async fn test_buffer_size_limit_bars() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let size = 4;
        let mut buffer = Buffer::new("TSLA".to_string(), size);

        // Add 5 bars to a buffer with size 3
        for i in 0..5 {
            let bar = values_to_bar(
                "TSLA",
                Utc::now(),
                200.0 + i as f64,
                205.0 + i as f64,
                210.0 + i as f64,
                195.0 + i as f64,
                1000.0,
            );
            buffer.add_bar(bar);
        }

        // Should only have 3 bars (the last 3)
        assert_eq!(buffer.bar_count(), size);

        // Verify we can get the bars
        assert!(buffer.get_bars().is_some());
        assert_eq!(buffer.get_bars().unwrap().len(), size);

        Ok(())
    }

    #[tokio::test]
    async fn test_buffer_symbol_access() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let buffer = Buffer::new("NVDA".to_string(), 10);

        assert_eq!(buffer.symbol(), "NVDA");

        Ok(())
    }

    #[tokio::test]
    async fn test_empty_buffer_returns_empty_vectors(
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let buffer = Buffer::new("GOOGL".to_string(), 5);

        assert!(buffer.get_bars().is_some());
        assert_eq!(buffer.get_bars().unwrap().len(), 0);

        assert!(buffer.get_trades().is_some());
        assert_eq!(buffer.get_trades().unwrap().len(), 0);

        Ok(())
    }
}
