import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { StellarRouteWebSocket, createWebSocketClient, WebSocketState, Clock, IWebSocket, IWebSocketFactory } from './websocket.js';

// ============================================================================
// Mocks
// ============================================================================

class MockWebSocket implements IWebSocket {
  static instances: MockWebSocket[] = [];
  
  readyState: number = 0; // CONNECTING
  onopen: ((event: any) => void) | null = null;
  onclose: ((event: any) => void) | null = null;
  onerror: ((event: any) => void) | null = null;
  onmessage: ((event: any) => void) | null = null;
  
  constructor(public url: string, private clock: Clock, connectDelay = 10) {
    MockWebSocket.instances.push(this);
    // Simulate async connection using the provided clock
    if (connectDelay >= 0) {
      this.clock.setTimeout(() => {
        if (this.readyState === 0) { // Still connecting
          this.readyState = 1; // OPEN
          this.onopen?.({ type: 'open' });
        }
      }, connectDelay);
    }
  }
  
  send(data: string) {
    // Echo back subscription confirmations
    try {
      const parsed = JSON.parse(data);
      if (parsed.action === 'subscribe') {
        this.clock.setTimeout(() => {
          this.onmessage?.({
            data: JSON.stringify({
              type: 'subscription_confirmed',
              subscription: parsed.subscription,
              timestamp: this.clock.now(),
            }),
          });
        }, 5);
      }
    } catch {
      // Ignore parse errors
    }
  }
  
  close(code = 1000, reason = '') {
    this.readyState = 3; // CLOSED
    this.onclose?.({ code, reason, type: 'close' });
  }
}

class MockWebSocketFactory implements IWebSocketFactory {
  public connectDelay = 10;
  constructor(private clock: Clock) {}
  create(url: string): IWebSocket {
    return new MockWebSocket(url, this.clock, this.connectDelay);
  }
}

class VirtualClock implements Clock {
  private currentTime = 1000000; // Start at a fixed time
  private timers: { id: number, callback: (...args: any[]) => void, time: number }[] = [];
  private nextId = 1;

  now() { return this.currentTime; }
  
  setTimeout(callback: (...args: any[]) => void, ms: number, ...args: any[]) {
    const id = this.nextId++;
    this.timers.push({ id, callback: () => callback(...args), time: this.currentTime + ms });
    this.timers.sort((a, b) => a.time - b.time);
    return id;
  }

  clearTimeout(id: number) {
    this.timers = this.timers.filter(t => t.id !== id);
  }

  tick(ms: number) {
    const targetTime = this.currentTime + ms;
    while (this.timers.length > 0 && this.timers[0].time <= targetTime) {
      const timer = this.timers.shift()!;
      this.currentTime = timer.time;
      timer.callback();
    }
    this.currentTime = targetTime;
  }
}

describe('StellarRouteWebSocket', () => {
  let client: StellarRouteWebSocket;
  let clock: Clock;
  let wsFactory: IWebSocketFactory;
  
  beforeEach(() => {
    MockWebSocket.instances = [];
    clock = {
      now: () => Date.now(),
      setTimeout: (cb, ms) => setTimeout(cb, ms),
      clearTimeout: (id) => clearTimeout(id),
    };
    wsFactory = new MockWebSocketFactory(clock);
    
    client = new StellarRouteWebSocket('ws://localhost:8080', {
      connectionTimeoutMs: 100,
      initialBackoffMs: 50,
      maxReconnectAttempts: 2,
      clock,
      webSocketFactory: wsFactory,
    });
  });
  
  afterEach(async () => {
    await client.disconnect();
  });
  
  describe('connection management', () => {
    it('should connect successfully', async () => {
      await client.connect();
      expect(client.isConnected()).toBe(true);
      expect(client.getState()).toBe('connected');
    });
    
    it('should emit connection state events', async () => {
      const states: WebSocketState[] = [];
      client.addEventListener((event) => {
        if (event.type === 'connection_state') {
          states.push(event.state);
        }
      });
      
      await client.connect();
      
      expect(states).toContain('connecting');
      expect(states).toContain('connected');
    });
    
    it('should disconnect cleanly', async () => {
      await client.connect();
      await client.disconnect();
      
      expect(client.isConnected()).toBe(false);
      expect(client.getState()).toBe('disconnected');
    });
    
    it('should not reconnect after explicit disconnect', async () => {
      await client.connect();
      const ws = MockWebSocket.instances[0];
      
      await client.disconnect();
      ws.close();
      
      // Wait for potential reconnect
      await new Promise((r) => setTimeout(r, 100));
      
      expect(MockWebSocket.instances.length).toBe(1);
    });
  });
  
  describe('subscription management', () => {
    it('should subscribe to quote updates', async () => {
      await client.connect();
      
      const id = client.subscribeToQuote('XLM', 'USDC');
      expect(id).toBeTruthy();
      expect(client.getSubscriptions().size).toBe(1);
    });
    
    it('should subscribe to orderbook updates', async () => {
      await client.connect();
      
      const id = client.subscribeToOrderbook('XLM', 'USDC');
      expect(id).toBeTruthy();
      expect(client.getSubscriptions().size).toBe(1);
    });
    
    it('should unsubscribe correctly', async () => {
      await client.connect();
      
      const id = client.subscribeToQuote('XLM', 'USDC');
      expect(client.getSubscriptions().size).toBe(1);
      
      client.unsubscribe(id);
      expect(client.getSubscriptions().size).toBe(0);
    });
    
    it('should unsubscribe from all subscriptions', async () => {
      await client.connect();
      
      client.subscribeToQuote('XLM', 'USDC');
      client.subscribeToOrderbook('XLM', 'USDC');
      expect(client.getSubscriptions().size).toBe(2);
      
      client.unsubscribeAll();
      expect(client.getSubscriptions().size).toBe(0);
    });
    
    it('should emit subscription confirmed event', async () => {
      await client.connect();
      
      const events: any[] = [];
      client.addEventListener((event) => events.push(event));
      
      client.subscribeToQuote('XLM', 'USDC');
      
      // Wait for mock to send confirmation
      await new Promise((r) => setTimeout(r, 20));
      
      expect(events.some((e: any) => e.type === 'subscription_confirmed')).toBe(true);
    });
  });
  
  describe('event handling', () => {
    it('should handle quote update events', async () => {
      await client.connect();
      const ws = MockWebSocket.instances[0];
      
      const events: any[] = [];
      client.addEventListener((event) => events.push(event));
      
      // Simulate server message
      ws.onmessage?.({
        data: JSON.stringify({
          type: 'quote_update',
          subscription: { type: 'quote', base: 'XLM', quote: 'USDC' },
          data: {
            base_asset: { asset_type: 'native' },
            quote_asset: { asset_code: 'USDC', asset_issuer: 'test' },
            amount: '100',
            price: '0.12',
            total: '12',
            quote_type: 'sell',
            path: [],
            timestamp: Date.now(),
          },
          timestamp: Date.now(),
        }),
      });
      
      expect(events.length).toBe(1);
      expect(events[0].type).toBe('quote_update');
    });
    
    it('should handle orderbook update events', async () => {
      await client.connect();
      const ws = MockWebSocket.instances[0];
      
      const events: any[] = [];
      client.addEventListener((event) => events.push(event));
      
      // Simulate server message
      ws.onmessage?.({
        data: JSON.stringify({
          type: 'orderbook_update',
          subscription: { type: 'orderbook', base: 'XLM', quote: 'USDC' },
          data: {
            base_asset: { asset_type: 'native' },
            quote_asset: { asset_code: 'USDC', asset_issuer: 'test' },
            bids: [],
            asks: [],
            timestamp: Date.now(),
          },
          timestamp: Date.now(),
        }),
      });
      
      expect(events.length).toBe(1);
      expect(events[0].type).toBe('orderbook_update');
    });
    
    it('should handle error events', async () => {
      await client.connect();
      const ws = MockWebSocket.instances[0];
      
      const events: any[] = [];
      client.addEventListener((event) => events.push(event));
      
      // Simulate server error
      ws.onmessage?.({
        data: JSON.stringify({
          type: 'error',
          code: 'invalid_subscription',
          message: 'Invalid trading pair',
          timestamp: Date.now(),
        }),
      });
      
      expect(events.length).toBe(1);
      expect(events[0].type).toBe('error');
      expect(events[0].code).toBe('invalid_subscription');
    });
    
    it('should remove event listener correctly', async () => {
      await client.connect();
      
      let callCount = 0;
      const listener = () => callCount++;
      
      const unsubscribe = client.addEventListener(listener);
      client.addEventListener(() => callCount++);
      
      client.unsubscribe(client.subscribeToQuote('XLM', 'USDC'));
      
      await new Promise((r) => setTimeout(r, 20));
      
      const countBefore = callCount;
      
      unsubscribe();
      
      // Further events should only increment once
      client.unsubscribe(client.subscribeToQuote('XLM', 'USDC'));
      
      expect(callCount).toBeLessThanOrEqual(countBefore + 1);
    });
  });
  
  describe('deterministic replay', () => {
    let vClock: VirtualClock;
    let vWsFactory: MockWebSocketFactory;
    
    beforeEach(() => {
      vClock = new VirtualClock();
      vWsFactory = new MockWebSocketFactory(vClock);
      client = new StellarRouteWebSocket('ws://localhost:8080', {
        clock: vClock,
        webSocketFactory: vWsFactory,
        initialBackoffMs: 1000,
        maxReconnectAttempts: 3,
      });
    });

    it('should handle connection timeout deterministically', async () => {
      vWsFactory.connectDelay = -1; // Never connect
      const connectPromise = client.connect();
      
      expect(client.getState()).toBe('connecting');
      
      // Tick to just before timeout (default 10000ms)
      vClock.tick(9999);
      expect(client.getState()).toBe('connecting');
      
      // Tick to timeout
      vClock.tick(1);
      
      await expect(connectPromise).rejects.toThrow('Connection timeout');
      expect(client.getState()).toBe('disconnected');
    });

    it('should handle reconnection backoff deterministically', async () => {
      const states: WebSocketState[] = [];
      client.addEventListener(e => {
        if (e.type === 'connection_state') states.push(e.state);
      });

      // Start connection
      client.connect().catch(() => {});
      vClock.tick(10); // MockWebSocket connects in 10ms
      expect(client.getState()).toBe('connected');
      
      // Simulate failure
      MockWebSocket.instances[0].close(1006, 'Abnormal Closure');
      expect(client.getState()).toBe('disconnected');
      
      // Should schedule reconnect in 1000ms
      vClock.tick(500);
      expect(client.getState()).toBe('disconnected');
      expect(MockWebSocket.instances.length).toBe(1);
      
      vClock.tick(500);
      expect(client.getState()).toBe('connecting');
      expect(MockWebSocket.instances.length).toBe(2);
      
      // Connect second attempt
      vClock.tick(10);
      expect(client.getState()).toBe('connected');
    });

    it('should handle multiple failures with exponential backoff', async () => {
      client.connect().catch(() => {});
      vClock.tick(10);
      
      // 1st failure
      MockWebSocket.instances[0].close(1006);
      vClock.tick(1000); // 1s backoff
      expect(client.getState()).toBe('connecting');
      
      // 2nd failure (immediate)
      MockWebSocket.instances[1].close(1006);
      expect(client.getState()).toBe('disconnected');
      
      vClock.tick(1999);
      expect(client.getState()).toBe('disconnected');
      vClock.tick(1); // 2s backoff
      expect(client.getState()).toBe('connecting');
      
      // 3rd failure
      MockWebSocket.instances[2].close(1006);
      vClock.tick(4000); // 4s backoff
      expect(client.getState()).toBe('connecting');
    });
  });
});

describe('createWebSocketClient', () => {
  it('should create a client with default options', () => {
    const client = createWebSocketClient();
    expect(client).toBeInstanceOf(StellarRouteWebSocket);
  });
});