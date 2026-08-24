import { describe, expect, it } from 'vitest';
import { createStubBridgeProvider } from './stub';

describe('stub bridge provider', () => {
  it('reports unavailable execution support', () => {
    const provider = createStubBridgeProvider();
    expect(
      provider.getAvailability({
        sourceChain: 'evm',
        destinationChain: 'stellar',
      })
    ).toMatchObject({
      kind: 'unsupported',
      code: 'no_backend_route',
    });
  });

  it('quote/prepare throw not-implemented style errors', async () => {
    const provider = createStubBridgeProvider();
    await expect(
      provider.quote({
        sourceChain: 'solana',
        destinationChain: 'stellar',
        amountIn: '1',
      })
    ).rejects.toMatchObject({ code: 'unsupported_capability' });
  });

  it('submit returns not_implemented without throwing', async () => {
    const provider = createStubBridgeProvider();
    await expect(
      provider.submit({ prepareId: 'x', signedPayload: {} })
    ).resolves.toMatchObject({ status: 'not_implemented' });
  });
});
