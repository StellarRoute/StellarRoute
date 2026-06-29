import { PostSwapSuccessScreen } from './PostSwapSuccessScreen';

export default {
  title: 'Swap / PostSwapSuccessScreen',
};

const NATIVE_ASSET = { asset_type: 'native' as const };
const USDC_ASSET = {
  asset_type: 'credit_alphanum4' as const,
  asset_code: 'USDC',
  asset_issuer: 'GDUKMGUGDZQK6YH...',
};

export const Default = () => (
  <div className="max-w-md mx-auto p-6 bg-background rounded-3xl border shadow-xl">
    <PostSwapSuccessScreen
      txHash="0x123abc456def7890123abc456def7890123abc456def"
      onDone={() => alert('Done clicked')}
      onSwapAgain={() => alert('Swap Again clicked')}
    />
  </div>
);

export const WithTradeParams = () => (
  <div className="max-w-md mx-auto p-6 bg-background rounded-3xl border shadow-xl">
    <PostSwapSuccessScreen
      txHash="0x123abc456def7890123abc456def7890123abc456def"
      tradeParams={{
        fromAmount: '100',
        fromAsset: 'XLM',
        toAmount: '50',
        toAsset: 'USDC',
        exchangeRate: '0.5000000',
        priceImpact: '0.12',
        minReceived: '49.5000000',
        networkFee: '0.01 XLM',
        routePath: [
          {
            from_asset: NATIVE_ASSET,
            to_asset: USDC_ASSET,
            price: '0.5000000',
            source: 'sdex',
          },
        ],
        walletAddress: 'GABC...',
      }}
      onDone={() => alert('Done clicked')}
      onSwapAgain={() => alert('Swap Again clicked')}
    />
  </div>
);
