export type ErrorCode =
  'INVALID_INPUT' | 'LIMIT_EXCEEDED' | 'OUT_OF_SCOPE' | 'CANCELLED';
export class ChefError extends Error {
  constructor(
    public readonly code: ErrorCode,
    message: string,
  ) {
    super(message);
    this.name = 'ChefError';
  }
}
