export interface CreatePoolParams {
  tokenA: string;
  tokenB: string;
  amountA: string;
  amountB: string;
  feeTier: number;
}

export interface CreatePoolResponse {
  txHash: string;
  callsUsed: number;
  remaining: number;
}

export class LiquidityCreatorClient {
  constructor(private options: { apiKey: string; baseUrl?: string }) {}

  async createPool(params: CreatePoolParams): Promise<CreatePoolResponse> {
    const res = await fetch(`${this.options.baseUrl ?? ""}/api/createPool`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "x-api-key": this.options.apiKey,
      },
      body: JSON.stringify(params),
    });

    if (!res.ok) {
      const err = await res.json();
      throw new Error(err.error ?? "Request failed");
    }

    return (await res.json()) as CreatePoolResponse;
  }
}
