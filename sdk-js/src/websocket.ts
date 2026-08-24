/**
 * WebSocket subscription API for live quote and orderbook updates.
 * 
 * Features:
 * - Subscribe/unsubscribe lifecycle
 * - Automatic reconnection with exponential backoff
 * - Typed event payloads
 * - Connection state management
 */

import type { PriceQuote, Orderbook } from './types.js';

// ============================================================================
// Types
// ============================================================================

export type WebSocketState = 'connecting' | 'connected' | 'disconnecting' | 'disconnected';

export interface Clock {
  now(): number;
  setTimeout(callback: (...args: any[]) => void, ms: number, ...args: any[]): any;
  clearTimeout(timeoutId: any): void;
}

export class DefaultClock implements Clock {
  now() { return Date.now(); }
  setTimeout(callback: (...args: any[]) => void, ms: number, ...args: any[]) {
    return setTimeout(callback, ms, ...args);
  }
  clearTimeout(timeoutId: any) {
    clearTimeout(timeoutId);
  }
}

export interface IWebSocket {
  readyState: number;
  onopen: ((event: any) => void) | null;
  onclose: ((event: any) => void) | null;
  onerror: ((event: any) => void) | null;
  onmessage: ((event: any) => void) | null;
  send(data: string): void;
  close(code?: number, reason?: string): void;
}

export interface IWebSocketFactory {
  create(url: string): IWebSocket;
}

export class DefaultWebSocketFactory implements IWebSocketFactory {
  create(url: string): IWebSocket {
    if (typeof WebSocket === 'undefined') {
      throw new Error('WebSocket is not available in this environment. Provide a WebSocketFactory or polyfill.');
    }
    return new (WebSocket as any)(url);
  }
}

export interface WebSocketOptions {
  /** Maximum reconnection attempts (default: 5) */
  maxReconnectAttempts?: number;
  /** Initial backoff delay in ms (default: 1000) */
  initialBackoffMs?: number;
  /** Maximum backoff delay in ms (default: 30000) */
  maxBackoffMs?: number;
  /** Backoff multiplier (default: 2) */
  backoffMultiplier?: number;
  /** Connection timeout in ms (default: 10000) */
  connectionTimeoutMs?: number;
  /** Enable debug logging */
  debug?: boolean;
  /** Custom clock for testing (default: DefaultClock) */
  clock?: Clock;
  /** Custom WebSocket factory for testing (default: DefaultWebSocketFactory) */
  webSocketFactory?: IWebSocketFactory;
}

export interface SubscriptionOptions {
  /** Subscription identifier for reference */
  id?: string;
}

export type SubscriptionType = 'quote' | 'orderbook';

export interface QuoteSubscription {
  type: 'quote';
  base: string;
  quote: string;
}

export interface OrderbookSubscription {
  type: 'orderbook';
  base: string;
  quote: string;
}

export type Subscription = QuoteSubscription | OrderbookSubscription;

export interface QuoteUpdateEvent {
  type: 'quote_update';
  subscription: QuoteSubscription;
  data: PriceQuote;
  timestamp: number;
}

export interface OrderbookUpdateEvent {
  type: 'orderbook_update';
  subscription: OrderbookSubscription;
  data: Orderbook;
  timestamp: number;
}

export interface SubscriptionConfirmedEvent {
  type: 'subscription_confirmed';
  subscription: Subscription;
  timestamp: number;
}

export interface SubscriptionRemovedEvent {
  type: 'subscription_removed';
  subscription: Subscription;
  timestamp: number;
}

export interface ErrorEvent {
  type: 'error';
  code: string;
  message: string;
  details?: unknown;
  timestamp: number;
}

export interface ConnectionStateEvent {
  type: 'connection_state';
  state: WebSocketState;
  timestamp: number;
}

export type WebSocketEvent = 
  | QuoteUpdateEvent 
  | OrderbookUpdateEvent
  | SubscriptionConfirmedEvent
  | SubscriptionRemovedEvent
  | ErrorEvent
  | ConnectionStateEvent;

export type EventListener = (event: WebSocketEvent) => void;

// ============================================================================
// WebSocket Client
// ============================================================================

export class StellarRouteWebSocket {
  private ws: IWebSocket | null = null;
  private readonly baseUrl: string;
  private readonly options: Required<WebSocketOptions>;
  private readonly clock: Clock;
  private readonly wsFactory: IWebSocketFactory;
  private state: WebSocketState = 'disconnected';
  private listeners: Set<EventListener> = new Set();
  private subscriptions: Map<string, Subscription> = new Map();
  private reconnectAttempts = 0;
  private reconnectTimeout: any = null;
  private connectionTimeout: any = null;
  private shouldReconnect = true;

  constructor(baseUrl = 'ws://localhost:8080', options?: WebSocketOptions) {
    this.baseUrl = baseUrl.replace(/\/$/, '').replace(/^http/, 'ws');
    this.clock = options?.clock ?? new DefaultClock();
    this.wsFactory = options?.webSocketFactory ?? new DefaultWebSocketFactory();
    this.options = {
      maxReconnectAttempts: options?.maxReconnectAttempts ?? 5,
      initialBackoffMs: options?.initialBackoffMs ?? 1000,
      maxBackoffMs: options?.maxBackoffMs ?? 30000,
      backoffMultiplier: options?.backoffMultiplier ?? 2,
      connectionTimeoutMs: options?.connectionTimeoutMs ?? 10000,
      debug: options?.debug ?? false,
      clock: this.clock,
      webSocketFactory: this.wsFactory,
    };
  }

  // ============================================================================
  // Connection Management
  // ============================================================================

  /**
   * Connect to the WebSocket server.
   * Automatically reconnects on disconnection unless disconnect() was called.
   */
  async connect(): Promise<void> {
    if (this.state === 'connected' || this.state === 'connecting') {
      return;
    }

    this.shouldReconnect = true;
    this.setState('connecting');

    return new Promise((resolve, reject) => {
      try {
        const wsUrl = `${this.baseUrl}/ws`;
        this.log('Connecting to', wsUrl);
        
        const currentWs = this.wsFactory.create(wsUrl);
        this.ws = currentWs;
        
        // Connection timeout
        this.connectionTimeout = this.clock.setTimeout(() => {
          if (this.ws === currentWs && this.ws.readyState !== 1 /* OPEN */) {
            this.log('Connection timeout');
            this.ws.close();
            reject(new Error('Connection timeout'));
          }
        }, this.options.connectionTimeoutMs);

        currentWs.onopen = () => {
          if (this.ws !== currentWs) return;
          
          this.clearConnectionTimeout();
          this.setState('connected');
          this.reconnectAttempts = 0;
          this.log('Connected');
          
          // Resubscribe to all active subscriptions
          this.resubscribeAll();
          resolve();
        };

        currentWs.onclose = (event) => {
          if (this.ws !== currentWs) return;
          
          this.clearConnectionTimeout();
          this.log('Disconnected', event?.code, event?.reason);
          
          this.setState('disconnected');

          // Attempt reconnection if not intentionally disconnected
          if (this.shouldReconnect && this.reconnectAttempts < this.options.maxReconnectAttempts) {
            this.scheduleReconnect();
          } else if (this.reconnectAttempts >= this.options.maxReconnectAttempts) {
            this.emit({
              type: 'error',
              code: 'reconnect_failed',
              message: 'Maximum reconnection attempts reached',
              timestamp: this.clock.now(),
            });
          }
        };

        currentWs.onerror = (error) => {
          if (this.ws !== currentWs) return;
          
          this.clearConnectionTimeout();
          this.log('WebSocket error', error);
          this.emit({
            type: 'error',
            code: 'websocket_error',
            message: 'WebSocket connection error',
            timestamp: this.clock.now(),
          });
        };

        currentWs.onmessage = (event) => {
          if (this.ws !== currentWs) return;
          this.handleMessage(event.data);
        };

      } catch (error) {
        this.clearConnectionTimeout();
        this.setState('disconnected');
        reject(error);
      }
    });
  }

  /**
   * Disconnect from the WebSocket server.
   * Prevents automatic reconnection.
   */
  async disconnect(): Promise<void> {
    this.shouldReconnect = false;
    this.clearReconnectTimeout();
    this.clearConnectionTimeout();
    
    if (this.ws) {
      this.setState('disconnecting');
      this.ws.close(1000, 'Client disconnect');
      this.ws = null;
    }
    
    this.setState('disconnected');
    this.subscriptions.clear();
  }

  /**
   * Get the current connection state.
   */
  getState(): WebSocketState {
    return this.state;
  }

  /**
   * Check if connected to the server.
   */
  isConnected(): boolean {
    return this.state === 'connected' && this.ws?.readyState === 1 /* OPEN */;
  }

  // ============================================================================
  // Subscription Management
  // ============================================================================

  /**
   * Subscribe to quote updates for a trading pair.
   */
  subscribeToQuote(base: string, quote: string, options?: SubscriptionOptions): string {
    const subscription: QuoteSubscription = { type: 'quote', base, quote };
    return this.subscribe(subscription, options);
  }

  /**
   * Subscribe to orderbook updates for a trading pair.
   */
  subscribeToOrderbook(base: string, quote: string, options?: SubscriptionOptions): string {
    const subscription: OrderbookSubscription = { type: 'orderbook', base, quote };
    return this.subscribe(subscription, options);
  }

  /**
   * Unsubscribe from updates.
   */
  unsubscribe(subscriptionId: string): void {
    const subscription = this.subscriptions.get(subscriptionId);
    if (!subscription) return;

    if (this.isConnected()) {
      this.send({
        action: 'unsubscribe',
        subscription,
      });
    }

    this.subscriptions.delete(subscriptionId);
    this.emit({
      type: 'subscription_removed',
      subscription,
      timestamp: this.clock.now(),
    });
  }

  /**
   * Unsubscribe from all subscriptions.
   */
  unsubscribeAll(): void {
    for (const id of this.subscriptions.keys()) {
      this.unsubscribe(id);
    }
  }

  /**
   * Get all active subscriptions.
   */
  getSubscriptions(): Map<string, Subscription> {
    return new Map(this.subscriptions);
  }

  // ============================================================================
  // Event Handling
  // ============================================================================

  /**
   * Add an event listener.
   */
  addEventListener(listener: EventListener): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  /**
   * Remove an event listener.
   */
  removeEventListener(listener: EventListener): void {
    this.listeners.delete(listener);
  }

  // ============================================================================
  // Private Methods
  // ============================================================================

  private subscribe(subscription: Subscription, options?: SubscriptionOptions): string {
    const id = options?.id ?? this.generateSubscriptionId(subscription);
    
    this.subscriptions.set(id, subscription);
    
    if (this.isConnected()) {
      this.send({
        action: 'subscribe',
        subscription,
      });
    }

    return id;
  }

  private generateSubscriptionId(subscription: Subscription): string {
    return `${subscription.type}:${subscription.base}/${subscription.quote}:${this.clock.now()}`;
  }

  private send(message: unknown): void {
    if (this.ws?.readyState === 1 /* OPEN */) {
      this.ws.send(JSON.stringify(message));
    }
  }

  private handleMessage(data: string): void {
    try {
      const message = JSON.parse(data);
      this.log('Received message', message);

      switch (message.type) {
        case 'quote_update':
          this.emit({
            type: 'quote_update',
            subscription: message.subscription,
            data: message.data,
            timestamp: message.timestamp ?? this.clock.now(),
          });
          break;

        case 'orderbook_update':
          this.emit({
            type: 'orderbook_update',
            subscription: message.subscription,
            data: message.data,
            timestamp: message.timestamp ?? this.clock.now(),
          });
          break;

        case 'subscription_confirmed':
          this.emit({
            type: 'subscription_confirmed',
            subscription: message.subscription,
            timestamp: message.timestamp ?? this.clock.now(),
          });
          break;

        case 'error':
          this.emit({
            type: 'error',
            code: message.code,
            message: message.message,
            details: message.details,
            timestamp: message.timestamp ?? this.clock.now(),
          });
          break;

        default:
          this.log('Unknown message type', message.type);
      }
    } catch (error) {
      this.log('Failed to parse message', error);
    }
  }

  private emit(event: WebSocketEvent): void {
    for (const listener of this.listeners) {
      try {
        listener(event);
      } catch (error) {
        this.log('Listener error', error);
      }
    }
  }

  private setState(state: WebSocketState): void {
    if (this.state === state) return;
    this.state = state;
    this.emit({
      type: 'connection_state',
      state,
      timestamp: this.clock.now(),
    });
  }

  private scheduleReconnect(): void {
    this.reconnectAttempts++;
    const backoff = Math.min(
      this.options.initialBackoffMs * Math.pow(this.options.backoffMultiplier, this.reconnectAttempts - 1),
      this.options.maxBackoffMs
    );
    
    this.log(`Scheduling reconnect attempt ${this.reconnectAttempts} in ${backoff}ms`);
    
    this.reconnectTimeout = this.clock.setTimeout(() => {
      this.connect().catch((error) => {
        this.log('Reconnect failed', error);
      });
    }, backoff);
  }

  private resubscribeAll(): void {
    for (const [id, subscription] of this.subscriptions) {
      this.log('Resubscribing to', id);
      this.send({
        action: 'subscribe',
        subscription,
      });
    }
  }

  private clearReconnectTimeout(): void {
    if (this.reconnectTimeout) {
      this.clock.clearTimeout(this.reconnectTimeout);
      this.reconnectTimeout = null;
    }
  }

  private clearConnectionTimeout(): void {
    if (this.connectionTimeout) {
      this.clock.clearTimeout(this.connectionTimeout);
      this.connectionTimeout = null;
    }
  }

  private log(...args: unknown[]): void {
    if (this.options.debug) {
      console.log('[StellarRouteWebSocket]', ...args);
    }
  }
}

// ============================================================================
// Utility Functions
// ============================================================================

/**
 * Create a WebSocket client with default options.
 */
export function createWebSocketClient(baseUrl?: string, options?: WebSocketOptions): StellarRouteWebSocket {
  return new StellarRouteWebSocket(baseUrl, options);
}