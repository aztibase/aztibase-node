import type { RpcResponse, RpcError } from './types.js';

export class AztibaseRpcError extends Error {
  code: number;
  data?: unknown;

  constructor(err: RpcError) {
    super(err.message);
    this.name = 'AztibaseRpcError';
    this.code = err.code;
    this.data = err.data;
  }
}

export class RpcClient {
  private url: string;
  private timeout: number;
  private id = 0;

  constructor(url: string, timeout = 10_000) {
    this.url = url;
    this.timeout = timeout;
  }

  async call<T>(method: string, params: unknown[] = []): Promise<T> {
    this.id++;
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), this.timeout);

    try {
      const res = await fetch(this.url, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          jsonrpc: '2.0',
          method,
          params,
          id: this.id,
        }),
        signal: controller.signal,
      });

      if (!res.ok) {
        throw new Error(`HTTP ${res.status}: ${res.statusText}`);
      }

      const json = (await res.json()) as RpcResponse<T>;

      if (json.error) {
        throw new AztibaseRpcError(json.error);
      }

      return json.result as T;
    } finally {
      clearTimeout(timer);
    }
  }

  async fetchMetrics(): Promise<Record<string, unknown>> {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), this.timeout);

    try {
      const res = await fetch(this.url.replace(/\/$/, '') + '/metrics/json', {
        signal: controller.signal,
      });
      return (await res.json()) as Record<string, unknown>;
    } finally {
      clearTimeout(timer);
    }
  }

  getUrl(): string {
    return this.url;
  }
}
