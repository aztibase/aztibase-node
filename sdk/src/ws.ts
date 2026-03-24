import type { RpcResponse } from './types.js';

type SubscriptionCallback = (data: unknown) => void;

export class AztibaseWs {
  private url: string;
  private ws: WebSocket | null = null;
  private id = 0;
  private pendingCalls = new Map<number, { resolve: (v: unknown) => void; reject: (e: Error) => void }>();
  private subscriptions = new Map<string, SubscriptionCallback>();
  private reconnectAttempts = 0;
  private maxReconnectAttempts: number;
  private reconnectDelay: number;
  private shouldReconnect = true;

  constructor(url: string, options?: { maxReconnectAttempts?: number; reconnectDelay?: number }) {
    this.url = url;
    this.maxReconnectAttempts = options?.maxReconnectAttempts ?? 5;
    this.reconnectDelay = options?.reconnectDelay ?? 3000;
  }

  async connect(): Promise<void> {
    return new Promise((resolve, reject) => {
      this.ws = new WebSocket(this.url);

      this.ws.onopen = () => {
        this.reconnectAttempts = 0;
        resolve();
      };

      this.ws.onerror = (event) => {
        if (this.reconnectAttempts === 0) {
          reject(new Error('WebSocket connection failed'));
        }
      };

      this.ws.onclose = () => {
        this.tryReconnect();
      };

      this.ws.onmessage = (event) => {
        this.handleMessage(event.data as string);
      };
    });
  }

  private handleMessage(raw: string) {
    let data: RpcResponse<unknown> & { method?: string; params?: { subscription?: string; result?: unknown } };
    try {
      data = JSON.parse(raw);
    } catch {
      return;
    }

    if (data.id != null && this.pendingCalls.has(data.id)) {
      const pending = this.pendingCalls.get(data.id)!;
      this.pendingCalls.delete(data.id);
      if (data.error) {
        pending.reject(new Error(data.error.message));
      } else {
        pending.resolve(data.result);
      }
      return;
    }

    if (data.params?.subscription && data.params?.result) {
      const cb = this.subscriptions.get(data.params.subscription);
      if (cb) cb(data.params.result);
    }
  }

  private async rpcCall<T>(method: string, params: unknown[] = []): Promise<T> {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
      throw new Error('WebSocket not connected');
    }

    this.id++;
    const id = this.id;

    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.pendingCalls.delete(id);
        reject(new Error(`RPC call ${method} timed out`));
      }, 10_000);

      this.pendingCalls.set(id, {
        resolve: (v) => { clearTimeout(timer); resolve(v as T); },
        reject: (e) => { clearTimeout(timer); reject(e); },
      });

      this.ws!.send(JSON.stringify({ jsonrpc: '2.0', method, params, id }));
    });
  }

  async subscribe(topic: string, callback: SubscriptionCallback): Promise<() => Promise<void>> {
    const subId = await this.rpcCall<string>('aztb_subscribe', [topic]);
    this.subscriptions.set(subId, callback);

    return async () => {
      this.subscriptions.delete(subId);
      try {
        await this.rpcCall('aztb_unsubscribe', [subId]);
      } catch {
        // ignore unsubscribe errors on close
      }
    };
  }

  private tryReconnect() {
    if (!this.shouldReconnect || this.reconnectAttempts >= this.maxReconnectAttempts) return;

    this.reconnectAttempts++;
    setTimeout(() => {
      this.connect().catch(() => {});
    }, this.reconnectDelay * this.reconnectAttempts);
  }

  close() {
    this.shouldReconnect = false;
    this.subscriptions.clear();
    for (const [, pending] of this.pendingCalls) {
      pending.reject(new Error('WebSocket closed'));
    }
    this.pendingCalls.clear();
    this.ws?.close();
    this.ws = null;
  }

  get connected(): boolean {
    return this.ws?.readyState === WebSocket.OPEN;
  }
}
