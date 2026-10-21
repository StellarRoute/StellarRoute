import React from 'react';

interface CardFaceProps {
  last4: string;
  expiry: string; // Format: MM/YY
  usdcAvailable: string;
  isFrozen?: boolean;
  className?: string;
}

/**
 * CardFace component displays a virtual card with essential information.
 * Shows last 4 digits, expiry, USDC balance, and settlement info.
 * When frozen, the card appears greyed out with a pause message.
 */
export const CardFace: React.FC<CardFaceProps> = ({
  last4,
  expiry,
  usdcAvailable,
  isFrozen = false,
  className = '',
}) => {
  return (
    <div
      className={`
        relative w-full max-w-md aspect-[1.586/1] rounded-2xl p-6
        bg-gradient-to-br from-slate-800 to-slate-900
        shadow-xl transition-all duration-300
        ${isFrozen ? 'opacity-60 grayscale' : 'hover:shadow-2xl'}
        ${className}
      `}
      role="img"
      aria-label={
        isFrozen
          ? 'Card spending is paused'
          : `Card ending in ${last4}, expires ${expiry}`
      }
    >
      {/* Card Logo/Brand Area */}
      <div className="flex justify-between items-start mb-8">
        <div className="text-white font-semibold text-lg">StellarRoute</div>
        <div className="text-white/60 text-sm">USDC Card</div>
      </div>

      {/* Card Number (masked with last 4) */}
      <div className="mb-6">
        <div className="flex gap-4 text-white/80 text-lg font-mono tracking-wider">
          <span>••••</span>
          <span>••••</span>
          <span>••••</span>
          <span className="text-white">{last4}</span>
        </div>
      </div>

      {/* Card Details Row */}
      <div className="flex justify-between items-end">
        <div className="space-y-1">
          <div className="text-white/60 text-xs uppercase tracking-wide">
            Expires
          </div>
          <div className="text-white font-mono text-base">{expiry}</div>
        </div>

        <div className="space-y-1 text-right">
          <div className="text-white/60 text-xs uppercase tracking-wide">
            Available
          </div>
          <div className="text-white font-semibold text-base">
            {usdcAvailable} USDC
          </div>
        </div>
      </div>

      {/* Settlement Info */}
      <div className="mt-4 pt-4 border-t border-white/10">
        <p className="text-white/50 text-xs">
          Charges settle in fiat via issuing partner
        </p>
      </div>

      {/* Frozen Overlay */}
      {isFrozen && (
        <div className="absolute inset-0 flex items-center justify-center bg-black/40 rounded-2xl backdrop-blur-sm">
          <div className="text-center">
            <div className="text-white text-xl font-semibold mb-2">
              Spending Paused
            </div>
            <div className="text-white/70 text-sm">
              Card is temporarily frozen
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

export default CardFace;
