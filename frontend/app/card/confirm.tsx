import React from 'react';

interface CardConfirmProps {
  merchant: string;
  fiatAmount: string;
  fiatCurrency: string;
  usdcGross: string;
  fee: string;
  exchangeRate: string;
  onConfirm?: () => void;
  onCancel?: () => void;
  isLoading?: boolean;
}

/**
 * CardConfirm displays transaction details before wallet signature.
 * Shows merchant, fiat amount, USDC breakdown, fee, and exchange rate.
 * This is shown BEFORE any signature is requested from the user.
 */
export const CardConfirm: React.FC<CardConfirmProps> = ({
  merchant,
  fiatAmount,
  fiatCurrency,
  usdcGross,
  fee,
  exchangeRate,
  onConfirm,
  onCancel,
  isLoading = false,
}) => {
  return (
    <div className="w-full max-w-lg mx-auto p-6 bg-white dark:bg-slate-900 rounded-xl shadow-lg">
      {/* Header */}
      <div className="mb-6 text-center">
        <h2 className="text-2xl font-bold text-slate-900 dark:text-white mb-2">
          Confirm Transaction
        </h2>
        <p className="text-slate-600 dark:text-slate-400 text-sm">
          Review details before signing
        </p>
      </div>

      {/* Merchant Info */}
      <div className="mb-6 p-4 bg-slate-50 dark:bg-slate-800 rounded-lg">
        <div className="text-xs uppercase tracking-wide text-slate-500 dark:text-slate-400 mb-1">
          Merchant
        </div>
        <div className="text-lg font-semibold text-slate-900 dark:text-white">
          {merchant}
        </div>
      </div>

      {/* Amount Details */}
      <div className="space-y-4 mb-6">
        {/* Fiat Amount */}
        <div className="flex justify-between items-center py-3 border-b border-slate-200 dark:border-slate-700">
          <span className="text-slate-600 dark:text-slate-400">
            Amount ({fiatCurrency})
          </span>
          <span className="text-lg font-semibold text-slate-900 dark:text-white">
            {fiatCurrency} {fiatAmount}
          </span>
        </div>

        {/* USDC Gross */}
        <div className="flex justify-between items-center py-3 border-b border-slate-200 dark:border-slate-700">
          <span className="text-slate-600 dark:text-slate-400">
            USDC Amount
          </span>
          <span className="text-lg font-semibold text-slate-900 dark:text-white">
            {usdcGross} USDC
          </span>
        </div>

        {/* Fee */}
        <div className="flex justify-between items-center py-3 border-b border-slate-200 dark:border-slate-700">
          <span className="text-slate-600 dark:text-slate-400">Fee</span>
          <span className="text-slate-900 dark:text-white">
            {fee} USDC
          </span>
        </div>

        {/* Total USDC */}
        <div className="flex justify-between items-center py-3 bg-blue-50 dark:bg-blue-900/20 rounded-lg px-4">
          <span className="font-semibold text-slate-700 dark:text-slate-300">
            Total USDC Required
          </span>
          <span className="text-xl font-bold text-blue-600 dark:text-blue-400">
            {(parseFloat(usdcGross) + parseFloat(fee)).toFixed(2)} USDC
          </span>
        </div>
      </div>

      {/* Exchange Rate */}
      <div className="mb-6 p-4 bg-slate-50 dark:bg-slate-800 rounded-lg">
        <div className="flex justify-between items-center">
          <span className="text-sm text-slate-600 dark:text-slate-400">
            Exchange Rate
          </span>
          <span className="text-sm font-mono text-slate-900 dark:text-white">
            1 USDC = {exchangeRate} {fiatCurrency}
          </span>
        </div>
      </div>

      {/* Action Buttons */}
      <div className="flex gap-3">
        <button
          onClick={onCancel}
          disabled={isLoading}
          className="flex-1 px-4 py-3 rounded-lg font-semibold
            bg-slate-200 dark:bg-slate-700
            text-slate-700 dark:text-slate-200
            hover:bg-slate-300 dark:hover:bg-slate-600
            disabled:opacity-50 disabled:cursor-not-allowed
            transition-colors duration-200"
          aria-label="Cancel transaction"
        >
          Cancel
        </button>

        <button
          onClick={onConfirm}
          disabled={isLoading}
          className="flex-1 px-4 py-3 rounded-lg font-semibold
            bg-blue-600 hover:bg-blue-700
            text-white
            disabled:opacity-50 disabled:cursor-not-allowed
            transition-colors duration-200
            relative"
          aria-label="Confirm transaction and sign"
        >
          {isLoading ? (
            <span className="flex items-center justify-center gap-2">
              <svg
                className="animate-spin h-5 w-5 text-white"
                xmlns="http://www.w3.org/2000/svg"
                fill="none"
                viewBox="0 0 24 24"
                aria-hidden="true"
              >
                <circle
                  className="opacity-25"
                  cx="12"
                  cy="12"
                  r="10"
                  stroke="currentColor"
                  strokeWidth="4"
                />
                <path
                  className="opacity-75"
                  fill="currentColor"
                  d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                />
              </svg>
              Processing...
            </span>
          ) : (
            'Confirm & Sign'
          )}
        </button>
      </div>

      {/* Warning Note */}
      <div className="mt-4 p-3 bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800 rounded-lg">
        <p className="text-xs text-amber-800 dark:text-amber-200">
          ⚠️ You will be prompted to sign this transaction with your wallet.
          Please verify all details before signing.
        </p>
      </div>
    </div>
  );
};

export default CardConfirm;
