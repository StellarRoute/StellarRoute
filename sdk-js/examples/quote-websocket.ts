/**
 * WebSocket Reconnection Example
 *
 * This example demonstrates how to use the StellarRouteWebSocket client with
 * automatic reconnection and exponential backoff. It shows:
 * - Connecting to the WebSocket endpoint
 * - Subscribing to quote updates
 * - Handling connection lifecycle events
 * - Graceful shutdown
 *
 * Run with: npx tsx examples/quote-websocket.ts
 * (Requires a running StellarRoute API server with WebSocket support)
 */

import { StellarRouteWebSocket, createWebSocketClient, WebSocketState } from '../src/index.js';

const API_URL = process.env.STELLARROUTE_WS_URL ?? 'ws://localhost:3000';

async function main(): Promise<void> {
  console.log('🔌 StellarRoute WebSocket Reconnection Example');
  console.log('==============================================\n');

  const client = createWebSocketClient(API_URL, {
    connectionTimeoutMs: 10_000,
    initialBackoffMs: 1_000,
    maxReconnectAttempts: 5,
  });

  let reconnectCount = 0;

  client.addEventListener((event) => {
    switch (event.type) {
      case 'connection_state': {
        const state = event.state;
        console.log(`📡 Connection state: ${state}`);
        if (state === 'connected') {
          console.log('✅ Connected to WebSocket server');
          reconnectCount = 0;
        } else if (state === 'connecting') {
          if (reconnectCount > 0) {
            console.log(`🔄 Reconnection attempt ${reconnectCount}...`);
          }
          reconnectCount++;
        } else if (state === 'disconnected') {
          console.log('❌ Disconnected from server');
        }
        break;
      }
      case 'quote_update': {
        const { data } = event;
        console.log(`💱 Quote update: ${data.base_asset.asset_code ?? 'XLM'} → ${data.quote_asset.asset_code}`);
        console.log(`   Price: ${data.price} | Amount: ${data.amount} | Total: ${data.total}`);
        break;
      }
      case 'orderbook_update': {
        const { data } = event;
        console.log(`📊 Orderbook update: ${data.bids.length} bids, ${data.asks.length} asks`);
        break;
      }
      case 'error': {
        console.error(`⚠️ Server error: ${event.code} — ${event.message}`);
        break;
      }
      case 'subscription_confirmed': {
        console.log(`✅ Subscription confirmed: ${event.subscription.type} ${event.subscription.base}/${event.subscription.quote}`);
        break;
      }
    }
  });

  try {
    console.log(`🔗 Connecting to ${API_URL}...`);
    await client.connect();

    const quoteSub = client.subscribeToQuote('native', 'USDC');
    console.log(`📝 Subscribed to XLM/USDC quotes (id: ${quoteSub})`);

    const orderbookSub = client.subscribeToOrderbook('native', 'USDC');
    console.log(`📝 Subscribed to XLM/USDC orderbook (id: ${orderbookSub})`);

    console.log('\n🎧 Listening for updates... Press Ctrl+C to exit\n');

    process.on('SIGINT', async () => {
      console.log('\n🛑 Shutting down...');
      client.unsubscribe(quoteSub);
      client.unsubscribe(orderbookSub);
      await client.disconnect();
      console.log('👋 Goodbye!');
      process.exit(0);
    });

    await new Promise(() => {});
  } catch (error) {
    console.error('💥 Fatal error:', error);
    process.exitCode = 1;
  }
}

main().catch((error) => {
  console.error('Failed to run example:', error);
  process.exitCode = 1;
});