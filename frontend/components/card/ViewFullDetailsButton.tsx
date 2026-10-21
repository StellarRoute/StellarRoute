import React from 'react';

interface ViewFullDetailsButtonProps {
  issuerPartnerName?: string;
  issuerPartnerUrl: string;
  onClick?: () => void;
  className?: string;
  variant?: 'primary' | 'secondary' | 'text';
  disabled?: boolean;
}

/**
 * ViewFullDetailsButton navigates to the issuer partner's platform.
 * Emphasizes that full card details (CVV, full number) are managed by the partner.
 * StellarRoute never holds keys or card PANs.
 */
export const ViewFullDetailsButton: React.FC<ViewFullDetailsButtonProps> = ({
  issuerPartnerName = 'issuing partner',
  issuerPartnerUrl,
  onClick,
  className = '',
  variant = 'primary',
  disabled = false,
}) => {
  const handleClick = () => {
    if (onClick) {
      onClick();
    }
    // Open issuer partner URL in new tab
    window.open(issuerPartnerUrl, '_blank', 'noopener,noreferrer');
  };

  const variantStyles = {
    primary:
      'bg-blue-600 hover:bg-blue-700 text-white shadow-md hover:shadow-lg',
    secondary:
      'bg-slate-200 dark:bg-slate-700 hover:bg-slate-300 dark:hover:bg-slate-600 text-slate-700 dark:text-slate-200',
    text: 'bg-transparent hover:bg-slate-100 dark:hover:bg-slate-800 text-blue-600 dark:text-blue-400',
  };

  return (
    <button
      onClick={handleClick}
      disabled={disabled}
      className={`
        inline-flex items-center justify-center gap-2
        px-4 py-2.5 rounded-lg font-medium
        transition-all duration-200
        disabled:opacity-50 disabled:cursor-not-allowed
        ${variantStyles[variant]}
        ${className}
      `}
      aria-label={`View full card details on ${issuerPartnerName} platform`}
      type="button"
    >
      <svg
        xmlns="http://www.w3.org/2000/svg"
        className="h-5 w-5"
        viewBox="0 0 20 20"
        fill="currentColor"
        aria-hidden="true"
      >
        <path d="M10 12a2 2 0 100-4 2 2 0 000 4z" />
        <path
          fillRule="evenodd"
          d="M.458 10C1.732 5.943 5.522 3 10 3s8.268 2.943 9.542 7c-1.274 4.057-5.064 7-9.542 7S1.732 14.057.458 10zM14 10a4 4 0 11-8 0 4 4 0 018 0z"
          clipRule="evenodd"
        />
      </svg>
      <span>View Full Details</span>
      <svg
        xmlns="http://www.w3.org/2000/svg"
        className="h-4 w-4"
        viewBox="0 0 20 20"
        fill="currentColor"
        aria-hidden="true"
      >
        <path d="M11 3a1 1 0 100 2h2.586l-6.293 6.293a1 1 0 101.414 1.414L15 6.414V9a1 1 0 102 0V4a1 1 0 00-1-1h-5z" />
        <path d="M5 5a2 2 0 00-2 2v8a2 2 0 002 2h8a2 2 0 002-2v-3a1 1 0 10-2 0v3H5V7h3a1 1 0 000-2H5z" />
      </svg>
    </button>
  );
};

/**
 * ViewFullDetailsDisclaimer explains that card details are managed by the partner.
 * Should be displayed near the ViewFullDetailsButton.
 */
export const ViewFullDetailsDisclaimer: React.FC<{
  issuerPartnerName?: string;
  className?: string;
}> = ({ issuerPartnerName = 'our issuing partner', className = '' }) => {
  return (
    <div
      className={`
        flex items-start gap-2 p-3 rounded-lg
        bg-blue-50 dark:bg-blue-900/20
        border border-blue-200 dark:border-blue-800
        ${className}
      `}
      role="note"
      aria-label="Card details disclaimer"
    >
      <svg
        xmlns="http://www.w3.org/2000/svg"
        className="h-5 w-5 text-blue-600 dark:text-blue-400 flex-shrink-0 mt-0.5"
        viewBox="0 0 20 20"
        fill="currentColor"
        aria-hidden="true"
      >
        <path
          fillRule="evenodd"
          d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7-4a1 1 0 11-2 0 1 1 0 012 0zM9 9a1 1 0 000 2v3a1 1 0 001 1h1a1 1 0 100-2v-3a1 1 0 00-1-1H9z"
          clipRule="evenodd"
        />
      </svg>
      <div className="text-sm text-blue-900 dark:text-blue-100">
        <p>
          Full card details (card number, CVV, PIN) are securely managed by{' '}
          <span className="font-semibold">{issuerPartnerName}</span>.
          StellarRoute never stores or has access to complete card information.
        </p>
      </div>
    </div>
  );
};

export default ViewFullDetailsButton;
