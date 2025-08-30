# Complete Shutdown Process

1. **Signal Capture** - The user presses Ctrl+C, and the signal handler in main.rs catches the signal.

2. **Signal Forwarding** - The signal handler calls `service_shutdown_tx.send()` to send the shutdown signal to the service.

3. **WebSocket Graceful Shutdown** - The Exchange layer receives the shutdown signal:
- Sends a Close frame to the Binance server
- Gracefully disconnects the WebSocket connection
- Returns `Ok(())` and does not reconnect.

4. **Data Processing Completed** - Data Processing Pipeline:
- Saves remaining data in the buffer
- Closes the data processing pipeline

5. **Service Layer Coordinated Shutdown** - MarketDataService:
- Waits for data collection and processing tasks to complete.
- "Market data service stopped normally"

6. **Application Layer Completion** - main.rs:
- service.start() returns normally.
- "Service stopped successfully"
- "Application stopped gracefully"