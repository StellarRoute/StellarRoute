import { useOptionalSettings } from "@/components/providers/settings-provider";
import {
  DEFAULT_LOCALE,
  getUserLocale,
  Locale,
} from "@/lib/formatting";

const SETTINGS_STORAGE_KEY = "stellar_route_settings";

export const SWAP_FALLBACK_LOCALE: Locale = DEFAULT_LOCALE;

type SupportedSwapLocale =
  | "en-US"
  | "zh-CN"
  | "es-ES"
  | "de-DE"
  | "fr-FR"
  | "ja-JP";

export type SwapTranslationKey =
  | "swap.card.title"
  | "swap.card.offlineBanner"
  | "swap.card.clearForm"
  | "swap.card.offlineQuoteError"
  | "swap.card.retryQuote"
  | "swap.pair.youPay"
  | "swap.pair.youReceive"
  | "swap.pair.amountPlaceholder"
  | "swap.pair.payAmountAriaLabel"
  | "swap.pair.receiveAmountAriaLabel"
  | "swap.pair.selectPayTokenAriaLabel"
  | "swap.pair.selectReceiveTokenAriaLabel"
  | "swap.pair.swapTokensAriaLabel"
  | "swap.pair.balance"
  | "swap.quote.rate"
  | "swap.quote.networkFee"
  | "swap.quote.priceImpact"
  | "swap.quote.minimumReceived"
  | "swap.quote.exchangeRateTooltip"
  | "swap.quote.minimumReceivedTooltip"
  | "swap.quote.networkFeeTooltip"
  | "swap.quote.exportJson"
  | "swap.quote.exportCsv"
  | "swap.quote.exportSuccess"
  | "swap.settings.buttonLabel"
  | "swap.settings.menuTitle"
  | "swap.settings.slippageTolerance"
  | "swap.simulation.errorTitle"
  | "swap.simulation.emptyState"
  | "swap.simulation.title"
  | "swap.simulation.slippageBadge"
  | "swap.simulation.highImpact"
  | "swap.simulation.expectedOutput"
  | "swap.simulation.minReceived"
  | "swap.simulation.fromSlippage"
  | "swap.simulation.effectiveRate"
  | "swap.simulation.priceImpact"
  | "swap.simulation.highImpactTitle"
  | "swap.simulation.highImpactBody"
  | "swap.route.title"
  | "swap.route.optimal"
  | "swap.route.showDetails"
  | "swap.route.expectedAmount"
  | "swap.route.expectedShort"
  | "swap.route.alternativeRoutes"
  | "swap.route.poolLabel"
  | "swap.route.altVenue"
  | "swap.fees.unavailableTitle"
  | "swap.fees.unavailableBody"
  | "swap.fees.title"
  | "swap.fees.protocolSection"
  | "swap.fees.networkSection"
  | "swap.fees.total"
  | "swap.fees.netOutput"
  | "swap.fees.routerFee.name"
  | "swap.fees.routerFee.description"
  | "swap.fees.poolFee.name"
  | "swap.fees.poolFee.description"
  | "swap.fees.baseFee.name"
  | "swap.fees.baseFee.description"
  | "swap.fees.operationFee.name"
  | "swap.fees.operationFee.description"
  | "swap.cta.reviewSwap"
  | "swap.cta.offline"
  | "swap.cta.selectTokens"
  | "swap.cta.enterAmount"
  | "swap.cta.invalidSlippage"
  | "swap.cta.loadingQuote"
  | "swap.cta.connectWallet"
  | "swap.cta.insufficientBalance"
  | "swap.cta.swapAnyway"
  | "swap.cta.swapping"
  | "swap.cta.errorFetchingQuote"
  | "swap.card.refreshQuote"
  | "swap.card.diagnostics"
  | "swap.card.outdated"
  | "swap.card.recoveringQuote"
  | "swap.card.recoveringQuoteCountdown"
  | "swap.card.cancelRetry"
  | "swap.card.sessionRestored"
  | "swap.card.poweredBy"
  | "swap.shortcuts.title"
  | "swap.shortcuts.openHelp"
  | "swap.shortcuts.closeHelp"
  | "swap.shortcuts.focusPayAmount"
  | "swap.shortcuts.focusReceiveAmount"
  | "swap.shortcuts.refreshQuote"
  | "swap.iconography.disclosure"
  | "swap.iconography.eyebrow"
  | "swap.iconography.title"
  | "swap.iconography.description"
  | "swap.iconography.venueTypes"
  | "swap.iconography.venueTypes.sdex"
  | "swap.iconography.venueTypes.hybrid"
  | "swap.iconography.transactionStates"
  | "swap.iconography.sizingNote"
  | "swap.iconography.assetFallbackNote"
  | "swap.a11y.quoteRefreshed"
  | "swap.a11y.quoteRefreshedGeneric"
  | "swap.a11y.quoteRefreshFailed"
  | "settings.page.title"
  | "settings.trade.title"
  | "settings.trade.description"
  | "settings.slippage.label"
  | "settings.slippage.typical"
  | "settings.slippage.error"
  | "settings.appearance.title"
  | "settings.appearance.description"
  | "settings.theme.label"
  | "settings.theme.placeholder"
  | "settings.theme.light"
  | "settings.theme.dark"
  | "settings.theme.system"
  | "settings.accentColor.label"
  | "settings.accentColor.description"
  | "settings.accentColor.custom"
  | "settings.accessibility.title"
  | "settings.accessibility.description"
  | "settings.textSize.label"
  | "settings.textSize.description"
  | "settings.textSize.preview.title"
  | "settings.textSize.preview.subtitle"
  | "settings.highContrast.label"
  | "settings.highContrast.description"
  | "settings.notifications.title"
  | "settings.notifications.description"
  | "settings.notifications.transactionLabel"
  | "settings.notifications.blocked"
  | "settings.notifications.unsupported"
  | "settings.notifications.enabledAria"
  | "settings.notifications.disabledAria"
  | "settings.notifications.blockedAria"
  | "settings.notifications.unsupportedAria"
  | "settings.reset.title"
  | "settings.reset.description"
  | "settings.reset.button"
  | "settings.reset.success"
  | "settings.panel.title"
  | "settings.panel.reset"
  | "settings.deadline.label"
  | "settings.deadline.min"
  | "settings.deadline.preset10m"
  | "settings.deadline.preset30m"
  | "settings.deadline.preset1h"
  | "settings.deadline.custom"
  | "settings.deadline.description"
  | "settings.slippage.custom"
  | "settings.slippage.deleteCustom"
  | "settings.slippage.lowWarning"
  | "settings.slippage.highWarning"
  | "settings.locale.title"
  | "settings.locale.description"
  | "settings.locale.example"
  | "settings.expert.mode"
  | "settings.expert.warning"
  | "settings.expert.bypass"
  | "settings.expert.bypassDescription"
  | "settings.expert.diagnostics"
  | "settings.expert.diagnosticsDescription"
  | "swap.confirm.review.heading"
  | "swap.confirm.review.description"
  | "swap.confirm.review.announcement"
  | "swap.confirm.pending.heading"
  | "swap.confirm.pending.description"
  | "swap.confirm.pending.announcement"
  | "swap.confirm.submitted.heading"
  | "swap.confirm.submitted.description"
  | "swap.confirm.submitted.announcement"
  | "swap.confirm.confirmed.heading"
  | "swap.confirm.confirmed.description"
  | "swap.confirm.confirmed.announcement"
  | "swap.confirm.failed.heading"
  | "swap.confirm.failed.description"
  | "swap.confirm.failed.announcement"
  | "swap.confirm.dropped.heading"
  | "swap.confirm.dropped.description"
  | "swap.confirm.dropped.announcement"
  | "swap.confirm.summary.youPay"
  | "swap.confirm.summary.youReceive"
  | "swap.confirm.summary.minReceived"
  | "swap.confirm.cta.confirmSwap"
  | "swap.confirm.cta.cancel"
  | "swap.confirm.cta.processing"
  | "swap.confirm.cta.done"
  | "swap.confirm.cta.tryAgain"
  | "swap.confirm.cta.dismiss"
  | "swap.confirm.cta.resubmit"
  | "orderbook.title"
  | "orderbook.description"
  | "orderbook.button.refresh"
  | "orderbook.header.price"
  | "orderbook.header.amount"
  | "orderbook.header.total"
  | "orderbook.pairs.loading.title"
  | "orderbook.pairs.loading.description"
  | "orderbook.pairs.error"
  | "orderbook.loading"
  | "orderbook.standby"
  | "orderbook.highlightedPair"
  | "orderbook.bids.title"
  | "orderbook.asks.title"
  | "orderbook.noBids"
  | "orderbook.noAsks"
  | "orderbook.row.bid.aria"
  | "orderbook.row.ask.aria"
  | "orderbook.depth.title"
  | "orderbook.depth.modeLabel"
  | "orderbook.depth.mode.compact"
  | "orderbook.depth.mode.detailed"
  | "orderbook.depth.densityLabel"
  | "orderbook.depth.density.low"
  | "orderbook.depth.density.high"
  | "orderbook.depth.perfLabel"
  | "orderbook.depth.fps"
  | "orderbook.depth.maxVolume"
  | "orderbook.depth.renderCost"
  | "orderbook.depth.memoryProfiler"
  | "orderbook.depth.uiJitter"
  | "orderbook.depth.activeStrategy"
  | "orderbook.depth.strategy.canvas"
  | "orderbook.depth.strategy.path"
  | "orderbook.depth.memoryStable"
  | "orderbook.depth.uiJitterZero"
  | "orderbook.depth.cost.canvas"
  | "orderbook.depth.cost.svg";

type SwapTranslations = Record<SwapTranslationKey, string>;

const SWAP_TRANSLATIONS_BASE: Record<
  "en-US" | "zh-CN" | "es-ES",
  SwapTranslations
> = {
  "en-US": {
    "swap.card.title": "Swap",
    "swap.card.offlineBanner":
      "You're offline. Quote refresh and swap submission are paused until your connection is restored.",
    "swap.card.clearForm": "Clear form",
    "swap.card.offlineQuoteError": "You are offline. Reconnect to refresh quote.",
    "swap.card.retryQuote": "Retry quote",
    "swap.pair.youPay": "You Pay",
    "swap.pair.youReceive": "You Receive",
    "swap.pair.amountPlaceholder": "0.00",
    "swap.pair.payAmountAriaLabel": "Pay amount",
    "swap.pair.receiveAmountAriaLabel": "Receive amount",
    "swap.pair.selectPayTokenAriaLabel": "Select token to pay",
    "swap.pair.selectReceiveTokenAriaLabel": "Select token to receive",
    "swap.pair.swapTokensAriaLabel": "Swap pay and receive tokens",
    "swap.pair.balance": "Balance: {amount}",
    "swap.quote.rate": "Rate",
    "swap.quote.networkFee": "Network Fee",
    "swap.quote.priceImpact": "Price Impact",
    "swap.quote.minimumReceived": "Minimum Received",
    "swap.quote.exchangeRateTooltip":
      "Current market rate for this trading pair inclusive of path routing.",
    "swap.quote.minimumReceivedTooltip":
      "Your transaction will revert if there is a large unfavorable price movement before it is confirmed.",
    "swap.quote.networkFeeTooltip":
      "Estimated cost to execute this transaction on the Stellar network.",
    "swap.quote.exportJson": "Export JSON",
    "swap.quote.exportCsv": "Export CSV",
    "swap.quote.exportSuccess": "Quote summary exported as {format}",
    "swap.settings.buttonLabel": "Settings",
    "swap.settings.menuTitle": "Transaction Settings",
    "swap.settings.slippageTolerance": "Slippage Tolerance",
    "swap.simulation.errorTitle": "Simulation Error",
    "swap.simulation.emptyState": "Enter an amount to see trade simulation",
    "swap.simulation.title": "Trade Simulation",
    "swap.simulation.slippageBadge": "{value}% slippage",
    "swap.simulation.highImpact": "High Impact",
    "swap.simulation.expectedOutput": "Expected Output",
    "swap.simulation.minReceived": "Min Received",
    "swap.simulation.fromSlippage": "-{amount} from slippage",
    "swap.simulation.effectiveRate": "Effective Rate",
    "swap.simulation.priceImpact": "Price Impact",
    "swap.simulation.highImpactTitle": "High Price Impact:",
    "swap.simulation.highImpactBody":
      "This trade may significantly affect the market price. Consider splitting into smaller orders.",
    "swap.route.title": "Best Route",
    "swap.route.optimal": "Optimal",
    "swap.route.showDetails": "Show route details",
    "swap.route.expectedAmount": "{amount} expected",
    "swap.route.expectedShort": "{amount} exp.",
    "swap.route.alternativeRoutes": "Alternative Routes",
    "swap.route.poolLabel": "AQUA Pool",
    "swap.route.altVenue": "SDEX",
    "swap.fees.unavailableTitle": "Fee Estimate",
    "swap.fees.unavailableBody":
      "Fee estimates are currently unavailable. Please try again later.",
    "swap.fees.title": "Fee Breakdown",
    "swap.fees.protocolSection": "Protocol Fees",
    "swap.fees.networkSection": "Network Costs",
    "swap.fees.total": "Total Fees",
    "swap.fees.netOutput": "Net Output",
    "swap.fees.routerFee.name": "Router Fee",
    "swap.fees.routerFee.description":
      "Fee for using StellarRoute aggregator",
    "swap.fees.poolFee.name": "Pool Fee",
    "swap.fees.poolFee.description":
      "Liquidity provider fee for AQUA pool",
    "swap.fees.baseFee.name": "Base Fee",
    "swap.fees.baseFee.description":
      "Stellar network base transaction fee",
    "swap.fees.operationFee.name": "Operation Fee",
    "swap.fees.operationFee.description":
      "Fee for path payment operations",
    "swap.cta.reviewSwap": "Review Swap",
    "swap.cta.offline": "Offline",
    "swap.cta.selectTokens": "Select tokens",
    "swap.cta.enterAmount": "Enter amount",
    "swap.cta.invalidSlippage": "Invalid slippage",
    "swap.cta.loadingQuote": "Loading quote...",
    "swap.cta.connectWallet": "Connect Wallet",
    "swap.cta.insufficientBalance": "Insufficient Balance",
    "swap.cta.swapAnyway": "Swap Anyway",
    "swap.cta.swapping": "Swapping...",
    "swap.cta.errorFetchingQuote": "Error fetching quote",
    "swap.card.refreshQuote": "Refresh quote",
    "swap.card.diagnostics": "View quote diagnostics",
    "swap.card.outdated": "Quote outdated — refresh for latest price",
    "swap.card.recoveringQuote": "Retrying quote...",
    "swap.card.recoveringQuoteCountdown": "Retrying quote in {seconds}s...",
    "swap.card.cancelRetry": "Cancel retry",
    "swap.card.sessionRestored":
      "Session restored — fetching a fresh quote before trading",
    "swap.card.poweredBy": "Powered by StellarRoute Aggregator",
    "swap.shortcuts.title": "Keyboard shortcuts",
    "swap.shortcuts.openHelp": "Open shortcut help",
    "swap.shortcuts.closeHelp": "Close modal",
    "swap.shortcuts.focusPayAmount": "Focus pay amount",
    "swap.shortcuts.focusReceiveAmount": "Focus receive amount",
    "swap.shortcuts.refreshQuote": "Refresh quote",
    "swap.iconography.disclosure": "Route and transaction icon legend",
    "swap.iconography.eyebrow": "Iconography System",
    "swap.iconography.title": "Route and Transaction Icons",
    "swap.iconography.description":
      "Consistent icons help users distinguish between venue types, hybrid routes, and transaction lifecycle states.",
    "swap.iconography.venueTypes": "Venue Types",
    "swap.iconography.venueTypes.sdex":
      "SDEX represents order book trades. AMM indicates liquidity pool swaps.",
    "swap.iconography.venueTypes.hybrid":
      "Hybrid routes combine both venue types for optimal routing.",
    "swap.iconography.transactionStates": "Transaction States",
    "swap.iconography.sizingNote":
      "Icons are sized for screen readability at 16/20/24px. Use light strokes for smaller badges and moderate stroke weight for larger route indicators.",
    "swap.iconography.assetFallbackNote":
      "Asset icons fall back to stable uppercase initials when a valid image source is unavailable.",
    "swap.a11y.quoteRefreshed": "Quote updated. {rate}",
    "swap.a11y.quoteRefreshedGeneric": "Quote updated.",
    "swap.a11y.quoteRefreshFailed": "Quote refresh failed. {message}",
    "settings.page.title": "Settings",
    "settings.trade.title": "Trade Settings",
    "settings.trade.description": "Configure your default trading parameters.",
    "settings.slippage.label": "Default Slippage Tolerance (%)",
    "settings.slippage.typical": "Typical: 0.5% - 1.0%",
    "settings.slippage.error": "Slippage must be between 0 and 50%",
    "settings.appearance.title": "Appearance",
    "settings.appearance.description": "Customize how StellarRoute looks on your device.",
    "settings.theme.label": "Theme",
    "settings.theme.placeholder": "Select theme",
    "settings.theme.light": "Light",
    "settings.theme.dark": "Dark",
    "settings.theme.system": "System",
    "settings.accentColor.label": "Accent Color",
    "settings.accentColor.description": "Applied to buttons, links, and other primary actions.",
    "settings.accentColor.custom": "Custom color:",
    "settings.accessibility.title": "Accessibility",
    "settings.accessibility.description": "Adjust text size and other accessibility options.",
    "settings.textSize.label": "Text Size",
    "settings.textSize.description": "Scale the interface font size up to 200% without breaking the layout.",
    "settings.textSize.preview.title": "Preview — StellarRoute",
    "settings.textSize.preview.subtitle": "Swap · Quote · Route · Settings",
    "settings.highContrast.label": "High Contrast Mode",
    "settings.highContrast.description": "Increases color contrast for improved readability and accessibility.",
    "settings.notifications.title": "Notifications",
    "settings.notifications.description": "Receive browser notifications for quote refreshes and swap status updates.",
    "settings.notifications.transactionLabel": "Transaction Notifications",
    "settings.notifications.blocked": "Notifications are blocked by your browser. Enable them in your browser settings to use this feature.",
    "settings.notifications.unsupported": "Your browser does not support desktop notifications.",
    "settings.notifications.enabledAria": "Browser notifications: enabled. Click to disable.",
    "settings.notifications.disabledAria": "Browser notifications: disabled. Click to enable.",
    "settings.notifications.blockedAria": "Browser notifications: blocked by browser. Change this in your browser settings.",
    "settings.notifications.unsupportedAria": "Browser notifications: not supported in this browser.",
    "settings.reset.title": "Reset Settings",
    "settings.reset.description": "Revert all settings to their original factory defaults.",
    "settings.reset.button": "Reset to Defaults",
    "settings.reset.success": "Settings reset to defaults",
    "settings.panel.title": "Settings",
    "settings.panel.reset": "Reset",
    "settings.deadline.label": "Transaction Deadline",
    "settings.deadline.min": "min",
    "settings.deadline.preset10m": "10m",
    "settings.deadline.preset30m": "30m",
    "settings.deadline.preset1h": "1h",
    "settings.deadline.custom": "Custom",
    "settings.deadline.description": "Transactions will revert if they are not confirmed within this timeframe.",
    "settings.slippage.custom": "Custom",
    "settings.slippage.deleteCustom": "Delete Custom Profile",
    "settings.slippage.lowWarning": "Your transaction may fail if the price moves unfavorably by more than {value}%.",
    "settings.slippage.highWarning": "High slippage increases the risk of frontrunning and getting a significantly worse price.",
    "settings.locale.title": "Language & Region",
    "settings.locale.description": "Choose your preferred language and number formatting. This affects how amounts, prices, and other numbers are displayed.",
    "settings.locale.example": "Example: {amount} · {percent}",
    "settings.expert.mode": "Expert Mode",
    "settings.expert.warning": "Expert Mode enables highly custom values and features. Be careful: high slippage limits can result in bad execution prices or frontrunning.",
    "settings.expert.bypass": "Bypass Confirmation Modal",
    "settings.expert.bypassDescription": "Execute transactions instantly with a single click. Useful for fast-moving markets.",
    "settings.expert.diagnostics": "Extended Route Diagnostics",
    "settings.expert.diagnosticsDescription": "Show raw liquidity pools, pool reserves, and exact multi-hop route metrics.",
    "swap.confirm.review.heading": "Review Swap",
    "swap.confirm.review.description": "Please review your swap details before confirming.",
    "swap.confirm.review.announcement": "Review your swap details.",
    "swap.confirm.pending.heading": "Waiting for wallet\u2026",
    "swap.confirm.pending.description": "Waiting for wallet signature. Please approve the transaction in your wallet.",
    "swap.confirm.pending.announcement": "Waiting for wallet signature.",
    "swap.confirm.submitted.heading": "Awaiting confirmation",
    "swap.confirm.submitted.description": "Transaction submitted, awaiting confirmation on the network.",
    "swap.confirm.submitted.announcement": "Transaction submitted, awaiting confirmation.",
    "swap.confirm.confirmed.heading": "Swap confirmed",
    "swap.confirm.confirmed.description": "Your swap has been confirmed on the Stellar network.",
    "swap.confirm.confirmed.announcement": "Swap confirmed successfully.",
    "swap.confirm.failed.heading": "Swap failed",
    "swap.confirm.failed.description": "The swap could not be completed. You can try again or dismiss.",
    "swap.confirm.failed.announcement": "Swap failed.",
    "swap.confirm.dropped.heading": "Transaction timed out",
    "swap.confirm.dropped.description": "The transaction was not confirmed within the deadline. You can resubmit or dismiss.",
    "swap.confirm.dropped.announcement": "Transaction timed out.",
    "swap.confirm.summary.youPay": "You pay",
    "swap.confirm.summary.youReceive": "You receive",
    "swap.confirm.summary.minReceived": "Min received",
    "swap.confirm.cta.confirmSwap": "Confirm Swap",
    "swap.confirm.cta.cancel": "Cancel",
    "swap.confirm.cta.processing": "Processing\u2026",
    "swap.confirm.cta.done": "Done",
    "swap.confirm.cta.tryAgain": "Try Again",
    "swap.confirm.cta.dismiss": "Dismiss",
    "swap.confirm.cta.resubmit": "Resubmit",
    "orderbook.title": "Orderbook",
    "orderbook.description":
      "Live bids and asks from the selected trading pair.",
    "orderbook.button.refresh": "Refresh",
    "orderbook.header.price": "Price",
    "orderbook.header.amount": "Amount",
    "orderbook.header.total": "Total",
    "orderbook.pairs.loading.title": "Loading markets",
    "orderbook.pairs.loading.description": "Fetching available trading pairs.",
    "orderbook.pairs.error":
      "⚠️ Market list indexer offline. Displaying local adaptive chart profiling tools.",
    "orderbook.loading": "Syncing order book grid...",
    "orderbook.standby": "Data feed standby mode active.",
    "orderbook.highlightedPair":
      "This pair is currently selected in the swap panel",
    "orderbook.bids.title": "Bids ({count})",
    "orderbook.asks.title": "Asks ({count})",
    "orderbook.noBids": "No bids available",
    "orderbook.noAsks": "No asks available",
    "orderbook.row.bid.aria":
      "Bid price {price}, amount {amount}, total {total}",
    "orderbook.row.ask.aria":
      "Ask price {price}, amount {amount}, total {total}",
    "orderbook.depth.title": "Market Depth",
    "orderbook.depth.modeLabel": "Mode",
    "orderbook.depth.mode.compact": "Compact (Canvas)",
    "orderbook.depth.mode.detailed": "Detailed (SVG)",
    "orderbook.depth.densityLabel": "Density Threshold",
    "orderbook.depth.density.low": "45p (SVG)",
    "orderbook.depth.density.high": "180p (Canvas)",
    "orderbook.depth.perfLabel": "Engine Performance",
    "orderbook.depth.fps": "{fps} FPS",
    "orderbook.depth.maxVolume": "Max Vol: {volume} units",
    "orderbook.depth.renderCost": "Render Cost",
    "orderbook.depth.memoryProfiler": "Memory Profiler",
    "orderbook.depth.uiJitter": "UI Jitter Bound",
    "orderbook.depth.activeStrategy": "Active Strategy",
    "orderbook.depth.strategy.canvas": "Adaptive Canvas Loop",
    "orderbook.depth.strategy.path": "Interactive Path",
    "orderbook.depth.memoryStable": "1.4 MB Stable",
    "orderbook.depth.uiJitterZero": "0% (Null Drops)",
    "orderbook.depth.cost.canvas": "~0.05ms [O(N)]",
    "orderbook.depth.cost.svg": "~0.22ms [DOM]",
  },
    "es-ES": {
    "swap.card.title": "Intercambiar",
    "swap.card.offlineBanner": "Estás desconectado. La actualización de cotizaciones y el envío de intercambios se pausarán hasta que se restaure la conexión.",
    "swap.card.clearForm": "Limpiar formulario",
    "swap.card.offlineQuoteError": "Estás desconectado. Reconéctate para actualizar la cotización.",
    "swap.card.retryQuote": "Reintentar cotización",
    "swap.pair.youPay": "Tú pagas",
    "swap.pair.youReceive": "Tú recibes",
    "swap.pair.amountPlaceholder": "0.00",
    "swap.pair.payAmountAriaLabel": "Cantidad a pagar",
    "swap.pair.receiveAmountAriaLabel": "Cantidad a recibir",
    "swap.pair.selectPayTokenAriaLabel": "Seleccionar token para pagar",
    "swap.pair.selectReceiveTokenAriaLabel": "Seleccionar token para recibir",
    "swap.pair.swapTokensAriaLabel": "Intercambiar tokens de pago y recepción",
    "swap.pair.balance": "Saldo: {amount}",
    "swap.quote.rate": "Tasa",
    "swap.quote.networkFee": "Comisión de red",
    "swap.quote.priceImpact": "Impacto en el precio",
    "swap.quote.minimumReceived": "Mínimo a recibir",
    "swap.quote.exchangeRateTooltip": "Tasa de mercado actual para este par de trading, incluido el enrutamiento de ruta.",
    "swap.quote.minimumReceivedTooltip": "La transacción se revertirá si hay un movimiento de precio desfavorable grande antes de que se confirme.",
    "swap.quote.networkFeeTooltip": "Costo estimado para ejecutar esta transacción en la red Stellar.",
    "swap.quote.exportJson": "Exportar JSON",
    "swap.quote.exportCsv": "Exportar CSV",
    "swap.quote.exportSuccess": "Resumen de cotización exportado como {format}",
    "swap.settings.buttonLabel": "Ajustes",
    "swap.settings.menuTitle": "Ajustes de transacción",
    "swap.settings.slippageTolerance": "Tolerancia de deslizamiento",
    "swap.simulation.errorTitle": "Error de simulación",
    "swap.simulation.emptyState": "Introduce una cantidad para ver la simulación",
    "swap.simulation.title": "Simulación de intercambio",
    "swap.simulation.slippageBadge": "{value}% deslizamiento",
    "swap.simulation.highImpact": "Alto impacto",
    "swap.simulation.expectedOutput": "Salida esperada",
    "swap.simulation.minReceived": "Mínimo recibido",
    "swap.simulation.fromSlippage": "-{amount} por deslizamiento",
    "swap.simulation.effectiveRate": "Tasa efectiva",
    "swap.simulation.priceImpact": "Impacto en el precio",
    "swap.simulation.highImpactTitle": "Alto impacto en el precio:",
    "swap.simulation.highImpactBody": "Esta operación puede afectar significativamente el precio del mercado. Considera dividirla en órdenes más pequeñas.",
    "swap.route.title": "Mejor ruta",
    "swap.route.optimal": "Óptimo",
    "swap.route.showDetails": "Mostrar detalles de la ruta",
    "swap.route.expectedAmount": "{amount} esperado",
    "swap.route.expectedShort": "{amount} esp.",
    "swap.route.alternativeRoutes": "Rutas alternativas",
    "swap.route.poolLabel": "Pool AQUA",
    "swap.route.altVenue": "SDEX",
    "swap.fees.unavailableTitle": "Estimación de comisiones",
    "swap.fees.unavailableBody": "Las estimaciones de comisiones no están disponibles actualmente. Inténtalo de nuevo más tarde.",
    "swap.fees.title": "Desglose de comisiones",
    "swap.fees.protocolSection": "Comisiones de protocolo",
    "swap.fees.networkSection": "Costos de red",
    "swap.fees.total": "Comisiones totales",
    "swap.fees.netOutput": "Salida neta",
    "swap.fees.routerFee.name": "Comisión de enrutador",
    "swap.fees.routerFee.description": "Comisión por usar el agregador StellarRoute",
    "swap.fees.poolFee.name": "Comisión del pool",
    "swap.fees.poolFee.description": "Comisión del proveedor de liquidez para el pool AQUA",
    "swap.fees.baseFee.name": "Comisión base",
    "swap.fees.baseFee.description": "Comisión base de transacción de la red Stellar",
    "swap.fees.operationFee.name": "Comisión de operación",
    "swap.fees.operationFee.description": "Comisión por operaciones de pago de ruta",
    "swap.cta.reviewSwap": "Revisar intercambio",
    "swap.cta.offline": "Desconectado",
    "swap.cta.selectTokens": "Seleccionar tokens",
    "swap.cta.enterAmount": "Introducir cantidad",
    "swap.cta.invalidSlippage": "Deslizamiento no válido",
    "swap.cta.loadingQuote": "Cargando cotización...",
    "swap.cta.connectWallet": "Conectar billetera",
    "swap.cta.insufficientBalance": "Saldo insuficiente",
    "swap.cta.swapAnyway": "Intercambiar de todos modos",
    "swap.cta.swapping": "Intercambiando...",
    "swap.cta.errorFetchingQuote": "Error al obtener cotización",
    "swap.card.refreshQuote": "Actualizar cotización",
    "swap.card.diagnostics": "View quote diagnostics",
    "swap.card.outdated": "Cotización desactualizada — actualiza para obtener el precio más reciente",
    "swap.card.recoveringQuote": "Reintentando cotización...",
    "swap.card.recoveringQuoteCountdown": "Reintentando cotización en {seconds}s...",
    "swap.card.cancelRetry": "Cancelar reintento",
    "swap.card.sessionRestored": "Sesión restaurada — obteniendo una cotización nueva antes de intercambiar",
    "swap.card.poweredBy": "Desarrollado por StellarRoute Aggregator",
    "swap.shortcuts.title": "Atajos de teclado",
    "swap.shortcuts.openHelp": "Abrir ayuda de atajos",
    "swap.shortcuts.closeHelp": "Cerrar modal",
    "swap.shortcuts.focusPayAmount": "Enfocar cantidad de pago",
    "swap.shortcuts.focusReceiveAmount": "Enfocar cantidad a recibir",
    "swap.shortcuts.refreshQuote": "Actualizar cotización",
    "swap.iconography.disclosure": "Leyenda de iconos de ruta y transacción",
    "swap.iconography.eyebrow": "Sistema de iconografía",
    "swap.iconography.title": "Iconos de ruta y transacción",
    "swap.iconography.description":
      "Los iconos coherentes ayudan a distinguir tipos de venue, rutas híbridas y estados del ciclo de vida de la transacción.",
    "swap.iconography.venueTypes": "Tipos de venue",
    "swap.iconography.venueTypes.sdex":
      "SDEX representa operaciones en el libro de órdenes. AMM indica intercambios en pools de liquidez.",
    "swap.iconography.venueTypes.hybrid":
      "Las rutas híbridas combinan ambos tipos de venue para un enrutamiento óptimo.",
    "swap.iconography.transactionStates": "Estados de transacción",
    "swap.iconography.sizingNote":
      "Los iconos están dimensionados para legibilidad en pantalla a 16/20/24px. Usa trazos ligeros para insignias pequeñas y un grosor moderado para indicadores de ruta más grandes.",
    "swap.iconography.assetFallbackNote":
      "Los iconos de activos usan iniciales en mayúsculas cuando no hay una imagen válida disponible.",
    "swap.a11y.quoteRefreshed": "Quote updated. {rate}",
    "swap.a11y.quoteRefreshedGeneric": "Quote updated.",
    "swap.a11y.quoteRefreshFailed": "Quote refresh failed. {message}",
    "settings.page.title": "Settings",
    "settings.trade.title": "Trade Settings",
    "settings.trade.description": "Configure your default trading parameters.",
    "settings.slippage.label": "Default Slippage Tolerance (%)",
    "settings.slippage.typical": "Typical: 0.5% - 1.0%",
    "settings.slippage.error": "Slippage must be between 0 and 50%",
    "settings.appearance.title": "Appearance",
    "settings.appearance.description": "Customize how StellarRoute looks on your device.",
    "settings.theme.label": "Theme",
    "settings.theme.placeholder": "Select theme",
    "settings.theme.light": "Light",
    "settings.theme.dark": "Dark",
    "settings.theme.system": "System",
    "settings.accentColor.label": "Accent Color",
    "settings.accentColor.description": "Applied to buttons, links, and other primary actions.",
    "settings.accentColor.custom": "Custom color:",
    "settings.accessibility.title": "Accessibility",
    "settings.accessibility.description": "Adjust text size and other accessibility options.",
    "settings.textSize.label": "Text Size",
    "settings.textSize.description": "Scale the interface font size up to 200% without breaking the layout.",
    "settings.textSize.preview.title": "Preview — StellarRoute",
    "settings.textSize.preview.subtitle": "Swap · Quote · Route · Settings",
    "settings.highContrast.label": "High Contrast Mode",
    "settings.highContrast.description": "Increases color contrast for improved readability and accessibility.",
    "settings.notifications.title": "Notifications",
    "settings.notifications.description": "Receive browser notifications for quote refreshes and swap status updates.",
    "settings.notifications.transactionLabel": "Transaction Notifications",
    "settings.notifications.blocked": "Notifications are blocked by your browser. Enable them in your browser settings to use this feature.",
    "settings.notifications.unsupported": "Your browser does not support desktop notifications.",
    "settings.notifications.enabledAria": "Browser notifications: enabled. Click to disable.",
    "settings.notifications.disabledAria": "Browser notifications: disabled. Click to enable.",
    "settings.notifications.blockedAria": "Browser notifications: blocked by browser. Change this in your browser settings.",
    "settings.notifications.unsupportedAria": "Browser notifications: not supported in this browser.",
    "settings.reset.title": "Reset Settings",
    "settings.reset.description": "Revert all settings to their original factory defaults.",
    "settings.reset.button": "Reset to Defaults",
    "settings.reset.success": "Settings reset to defaults",
    "settings.panel.title": "Settings",
    "settings.panel.reset": "Reset",
    "settings.deadline.label": "Transaction Deadline",
    "settings.deadline.min": "min",
    "settings.deadline.preset10m": "10m",
    "settings.deadline.preset30m": "30m",
    "settings.deadline.preset1h": "1h",
    "settings.deadline.custom": "Custom",
    "settings.deadline.description": "Transactions will revert if they are not confirmed within this timeframe.",
    "settings.slippage.custom": "Custom",
    "settings.slippage.deleteCustom": "Delete Custom Profile",
    "settings.slippage.lowWarning": "Your transaction may fail if the price moves unfavorably by more than {value}%.",
    "settings.slippage.highWarning": "High slippage increases the risk of frontrunning and getting a significantly worse price.",
    "settings.locale.title": "Language & Region",
    "settings.locale.description": "Choose your preferred language and number formatting. This affects how amounts, prices, and other numbers are displayed.",
    "settings.locale.example": "Example: {amount} · {percent}",
    "settings.expert.mode": "Expert Mode",
    "settings.expert.warning": "Expert Mode enables highly custom values and features. Be careful: high slippage limits can result in bad execution prices or frontrunning.",
    "settings.expert.bypass": "Bypass Confirmation Modal",
    "settings.expert.bypassDescription": "Execute transactions instantly with a single click. Useful for fast-moving markets.",
    "settings.expert.diagnostics": "Extended Route Diagnostics",
    "settings.expert.diagnosticsDescription": "Show raw liquidity pools, pool reserves, and exact multi-hop route metrics.",
    "swap.confirm.review.heading": "Revisar intercambio",
    "swap.confirm.review.description": "Por favor, revisa los detalles de tu intercambio antes de confirmar.",
    "swap.confirm.review.announcement": "Revisa los detalles del intercambio.",
    "swap.confirm.pending.heading": "Esperando a la cartera\u2026",
    "swap.confirm.pending.description": "Esperando la firma de la cartera. Por favor, aprueba la transacci\u00f3n en tu cartera.",
    "swap.confirm.pending.announcement": "Esperando la firma de la cartera.",
    "swap.confirm.submitted.heading": "Esperando confirmaci\u00f3n",
    "swap.confirm.submitted.description": "Transacci\u00f3n enviada, esperando confirmaci\u00f3n en la red.",
    "swap.confirm.submitted.announcement": "Transacci\u00f3n enviada, esperando confirmaci\u00f3n.",
    "swap.confirm.confirmed.heading": "Intercambio confirmado",
    "swap.confirm.confirmed.description": "Tu intercambio ha sido confirmado en la red Stellar.",
    "swap.confirm.confirmed.announcement": "Intercambio confirmado correctamente.",
    "swap.confirm.failed.heading": "Intercambio fallido",
    "swap.confirm.failed.description": "El intercambio no pudo completarse. Puedes volver a intentarlo o descartarlo.",
    "swap.confirm.failed.announcement": "Intercambio fallido.",
    "swap.confirm.dropped.heading": "Transacci\u00f3n agotada",
    "swap.confirm.dropped.description": "La transacci\u00f3n no fue confirmada antes del plazo. Puedes reenviarla o descartarla.",
    "swap.confirm.dropped.announcement": "Transacci\u00f3n agotada.",
    "swap.confirm.summary.youPay": "T\u00fa pagas",
    "swap.confirm.summary.youReceive": "T\u00fa recibes",
    "swap.confirm.summary.minReceived": "M\u00ednimo recibido",
    "swap.confirm.cta.confirmSwap": "Confirmar intercambio",
    "swap.confirm.cta.cancel": "Cancelar",
    "swap.confirm.cta.processing": "Procesando\u2026",
    "swap.confirm.cta.done": "Hecho",
    "swap.confirm.cta.tryAgain": "Volver a intentar",
    "swap.confirm.cta.dismiss": "Descartar",
    "swap.confirm.cta.resubmit": "Reenviar",
    "orderbook.title": "Libro de órdenes",
    "orderbook.description":
      "Ofertas y demandas en vivo del par de trading seleccionado.",
    "orderbook.button.refresh": "Actualizar",
    "orderbook.header.price": "Precio",
    "orderbook.header.amount": "Cantidad",
    "orderbook.header.total": "Total",
    "orderbook.pairs.loading.title": "Cargando mercados",
    "orderbook.pairs.loading.description":
      "Obteniendo pares de trading disponibles.",
    "orderbook.pairs.error":
      "⚠️ Indexador de lista de mercado fuera de línea. Mostrando herramientas de perfilado de gráficos adaptativos locales.",
    "orderbook.loading": "Sincronizando cuadrícula del libro de órdenes...",
    "orderbook.standby": "Modo de espera de alimentación de datos activo.",
    "orderbook.highlightedPair":
      "Este par está seleccionado actualmente en el panel de intercambio",
    "orderbook.bids.title": "Ofertas ({count})",
    "orderbook.asks.title": "Demandas ({count})",
    "orderbook.noBids": "No hay ofertas disponibles",
    "orderbook.noAsks": "No hay demandas disponibles",
    "orderbook.row.bid.aria":
      "Oferta precio {price}, cantidad {amount}, total {total}",
    "orderbook.row.ask.aria":
      "Demanda precio {price}, cantidad {amount}, total {total}",
    "orderbook.depth.title": "Profundidad de mercado",
    "orderbook.depth.modeLabel": "Modo",
    "orderbook.depth.mode.compact": "Compacto (Canvas)",
    "orderbook.depth.mode.detailed": "Detallado (SVG)",
    "orderbook.depth.densityLabel": "Umbral de densidad",
    "orderbook.depth.density.low": "45p (SVG)",
    "orderbook.depth.density.high": "180p (Canvas)",
    "orderbook.depth.perfLabel": "Rendimiento del motor",
    "orderbook.depth.fps": "{fps} FPS",
    "orderbook.depth.maxVolume": "Vol. máx.: {volume} unidades",
    "orderbook.depth.renderCost": "Costo de renderizado",
    "orderbook.depth.memoryProfiler": "Perfilador de memoria",
    "orderbook.depth.uiJitter": "Límite de jitter de UI",
    "orderbook.depth.activeStrategy": "Estrategia activa",
    "orderbook.depth.strategy.canvas": "Bucle Canvas adaptativo",
    "orderbook.depth.strategy.path": "Ruta interactiva",
    "orderbook.depth.memoryStable": "1.4 MB estable",
    "orderbook.depth.uiJitterZero": "0% (sin pérdidas)",
    "orderbook.depth.cost.canvas": "~0.05ms [O(N)]",
    "orderbook.depth.cost.svg": "~0.22ms [DOM]",
  },
  "zh-CN": {
    "swap.card.title": "兑换",
    "swap.card.offlineBanner":
      "你当前处于离线状态。报价刷新和兑换提交已暂停，连接恢复后会继续。",
    "swap.card.clearForm": "清空表单",
    "swap.card.offlineQuoteError": "你当前处于离线状态。恢复网络后再刷新报价。",
    "swap.card.retryQuote": "重试报价",
    "swap.pair.youPay": "你支付",
    "swap.pair.youReceive": "你收到",
    "swap.pair.amountPlaceholder": "0.00",
    "swap.pair.payAmountAriaLabel": "支付数量",
    "swap.pair.receiveAmountAriaLabel": "接收数量",
    "swap.pair.selectPayTokenAriaLabel": "选择支付代币",
    "swap.pair.selectReceiveTokenAriaLabel": "选择接收代币",
    "swap.pair.swapTokensAriaLabel": "交换支付和接收代币",
    "swap.pair.balance": "余额：{amount}",
    "swap.quote.rate": "汇率",
    "swap.quote.networkFee": "网络费用",
    "swap.quote.priceImpact": "价格影响",
    "swap.quote.minimumReceived": "最少收到",
    "swap.quote.exchangeRateTooltip": "包含路径路由影响的当前市场汇率。",
    "swap.quote.minimumReceivedTooltip":
      "若确认前出现不利的大幅价格波动，交易将回滚。",
    "swap.quote.networkFeeTooltip": "在 Stellar 网络执行该交易的预计成本。",
    "swap.quote.exportJson": "导出 JSON",
    "swap.quote.exportCsv": "导出 CSV",
    "swap.quote.exportSuccess": "报价摘要已导出为 {format}",
    "swap.settings.buttonLabel": "设置",
    "swap.settings.menuTitle": "交易设置",
    "swap.settings.slippageTolerance": "滑点容忍度",
    "swap.simulation.errorTitle": "模拟失败",
    "swap.simulation.emptyState": "输入数量后即可查看交易模拟",
    "swap.simulation.title": "交易模拟",
    "swap.simulation.slippageBadge": "滑点 {value}%",
    "swap.simulation.highImpact": "高影响",
    "swap.simulation.expectedOutput": "预计到账",
    "swap.simulation.minReceived": "最少收到",
    "swap.simulation.fromSlippage": "因滑点减少 {amount}",
    "swap.simulation.effectiveRate": "实际汇率",
    "swap.simulation.priceImpact": "价格影响",
    "swap.simulation.highImpactTitle": "价格影响较高：",
    "swap.simulation.highImpactBody":
      "这笔交易可能会明显影响市场价格，建议拆分成更小的订单执行。",
    "swap.route.title": "最佳路径",
    "swap.route.optimal": "最优",
    "swap.route.showDetails": "显示路径详情",
    "swap.route.expectedAmount": "预计到账 {amount}",
    "swap.route.expectedShort": "预计 {amount}",
    "swap.route.alternativeRoutes": "备选路径",
    "swap.route.poolLabel": "AQUA 池",
    "swap.route.altVenue": "SDEX",
    "swap.fees.unavailableTitle": "费用估算",
    "swap.fees.unavailableBody": "暂时无法获取费用估算，请稍后再试。",
    "swap.fees.title": "费用拆分",
    "swap.fees.protocolSection": "协议费用",
    "swap.fees.networkSection": "网络成本",
    "swap.fees.total": "总费用",
    "swap.fees.netOutput": "净到账",
    "swap.fees.routerFee.name": "路由费",
    "swap.fees.routerFee.description": "使用 StellarRoute 聚合器的费用",
    "swap.fees.poolFee.name": "资金池费",
    "swap.fees.poolFee.description": "AQUA 池流动性提供者费用",
    "swap.fees.baseFee.name": "基础费",
    "swap.fees.baseFee.description": "Stellar 网络基础交易费用",
    "swap.fees.operationFee.name": "操作费",
    "swap.fees.operationFee.description": "路径支付操作产生的费用",
    "swap.cta.reviewSwap": "检查兑换",
    "swap.cta.offline": "离线",
    "swap.cta.selectTokens": "选择代币",
    "swap.cta.enterAmount": "输入数量",
    "swap.cta.invalidSlippage": "滑点无效",
    "swap.cta.loadingQuote": "正在获取报价...",
    "swap.cta.connectWallet": "连接钱包",
    "swap.cta.insufficientBalance": "余额不足",
    "swap.cta.swapAnyway": "仍要兑换",
    "swap.cta.swapping": "兑换中...",
    "swap.cta.errorFetchingQuote": "获取报价失败",
    "swap.card.refreshQuote": "刷新报价",
    "swap.card.diagnostics": "查看报价诊断信息",
    "swap.card.outdated": "报价已过期——请刷新获取最新价格",
    "swap.card.recoveringQuote": "正在重试报价...",
    "swap.card.recoveringQuoteCountdown": "{seconds} 秒后重试报价...",
    "swap.card.cancelRetry": "取消重试",
    "swap.card.sessionRestored": "会话已恢复——正在获取最新报价后再交易",
    "swap.card.poweredBy": "由 StellarRoute 聚合器提供支持",
    "swap.shortcuts.title": "键盘快捷键",
    "swap.shortcuts.openHelp": "打开快捷键帮助",
    "swap.shortcuts.closeHelp": "关闭弹窗",
    "swap.shortcuts.focusPayAmount": "聚焦支付数量",
    "swap.shortcuts.focusReceiveAmount": "聚焦接收数量",
    "swap.shortcuts.refreshQuote": "刷新报价",
    "swap.iconography.disclosure": "路径与交易图标图例",
    "swap.iconography.eyebrow": "图标系统",
    "swap.iconography.title": "路径与交易图标",
    "swap.iconography.description":
      "一致的图标可帮助用户区分交易场所类型、混合路径和交易生命周期状态。",
    "swap.iconography.venueTypes": "交易场所类型",
    "swap.iconography.venueTypes.sdex":
      "SDEX 表示订单簿交易，AMM 表示流动性池兑换。",
    "swap.iconography.venueTypes.hybrid":
      "混合路径会结合两种交易场所以实现更优路由。",
    "swap.iconography.transactionStates": "交易状态",
    "swap.iconography.sizingNote":
      "图标按 16/20/24 像素优化可读性。较小徽章使用细描边，较大路径指示器使用中等描边粗细。",
    "swap.iconography.assetFallbackNote":
      "当有效图片源不可用时，资产图标会回退为稳定的大写首字母。",
    "swap.a11y.quoteRefreshed": "报价已更新。{rate}",
    "swap.a11y.quoteRefreshedGeneric": "报价已更新。",
    "swap.a11y.quoteRefreshFailed": "报价刷新失败。{message}",
    "settings.page.title": "设置",
    "settings.trade.title": "交易设置",
    "settings.trade.description": "配置你的默认交易参数。",
    "settings.slippage.label": "默认滑点容忍度 (%)",
    "settings.slippage.typical": "典型值：0.5% - 1.0%",
    "settings.slippage.error": "滑点必须在 0 到 50% 之间",
    "settings.appearance.title": "外观",
    "settings.appearance.description": "自定义 StellarRoute 在你设备上的显示方式。",
    "settings.theme.label": "主题",
    "settings.theme.placeholder": "选择主题",
    "settings.theme.light": "浅色",
    "settings.theme.dark": "深色",
    "settings.theme.system": "跟随系统",
    "settings.accentColor.label": "强调色",
    "settings.accentColor.description": "应用于按钮、链接和其他主要操作。",
    "settings.accentColor.custom": "自定义颜色：",
    "settings.accessibility.title": "辅助功能",
    "settings.accessibility.description": "调整文本大小和其他辅助功能选项。",
    "settings.textSize.label": "文本大小",
    "settings.textSize.description": "将界面字体大小缩放至 200%，且不会破坏布局。",
    "settings.textSize.preview.title": "预览 — StellarRoute",
    "settings.textSize.preview.subtitle": "兑换 · 报价 · 路径 · 设置",
    "settings.highContrast.label": "高对比度模式",
    "settings.highContrast.description": "增加颜色对比度，提高可读性和辅助功能。",
    "settings.notifications.title": "通知",
    "settings.notifications.description": "接收报价刷新和交易状态更新的浏览器通知。",
    "settings.notifications.transactionLabel": "交易通知",
    "settings.notifications.blocked": "通知被你的浏览器阻止。请在浏览器设置中启用通知以使用此功能。",
    "settings.notifications.unsupported": "你的浏览器不支持桌面通知。",
    "settings.notifications.enabledAria": "浏览器通知：已启用。点击禁用。",
    "settings.notifications.disabledAria": "浏览器通知：已禁用。点击启用。",
    "settings.notifications.blockedAria": "浏览器通知：被浏览器阻止。请在浏览器设置中更改。",
    "settings.notifications.unsupportedAria": "浏览器通知：此浏览器不支持。",
    "settings.reset.title": "重置设置",
    "settings.reset.description": "将所有设置恢复为原始出厂默认值。",
    "settings.reset.button": "重置为默认值",
    "settings.reset.success": "设置已重置为默认值",
    "settings.panel.title": "设置",
    "settings.panel.reset": "重置",
    "settings.deadline.label": "交易期限",
    "settings.deadline.min": "分钟",
    "settings.deadline.preset10m": "10分钟",
    "settings.deadline.preset30m": "30分钟",
    "settings.deadline.preset1h": "1小时",
    "settings.deadline.custom": "自定义",
    "settings.deadline.description": "如果在此时间范围内未确认，交易将回滚。",
    "settings.slippage.custom": "自定义",
    "settings.slippage.deleteCustom": "删除自定义配置",
    "settings.slippage.lowWarning": "如果价格对您不利的波动超过 {value}%，您的交易可能会失败。",
    "settings.slippage.highWarning": "高滑点会增加被抢先交易和获得明显更差价格的风险。",
    "settings.locale.title": "语言和地区",
    "settings.locale.description": "选择你的首选语言和数字格式。这会影响金额、价格和其他数字的显示方式。",
    "settings.locale.example": "示例：{amount} · {percent}",
    "settings.expert.mode": "专家模式",
    "settings.expert.warning": "专家模式允许高度自定义的数值和功能。请注意：较高的滑点限制可能会导致糟糕的执行价格或遭遇抢先交易。",
    "settings.expert.bypass": "绕过确认弹窗",
    "settings.expert.bypassDescription": "单击即可立即执行交易。适用于瞬息万变的市场。",
    "settings.expert.diagnostics": "扩展路径诊断",
    "settings.expert.diagnosticsDescription": "显示原始流动性池、池储备量和精确的多跳路径指标。",
    "swap.confirm.review.heading": "检查兑换",
    "swap.confirm.review.description": "确认前请检查兑换详情。",
    "swap.confirm.review.announcement": "请检查兑换详情。",
    "swap.confirm.pending.heading": "等待钱包\u2026",
    "swap.confirm.pending.description": "等待钱包签名，请在钱包中批准此交易。",
    "swap.confirm.pending.announcement": "等待钱包签名。",
    "swap.confirm.submitted.heading": "等待确认",
    "swap.confirm.submitted.description": "交易已提交，等待网络确认。",
    "swap.confirm.submitted.announcement": "交易已提交，等待确认。",
    "swap.confirm.confirmed.heading": "兑换已确认",
    "swap.confirm.confirmed.description": "您的兑换已在 Stellar 网络上确认。",
    "swap.confirm.confirmed.announcement": "兑换已成功确认。",
    "swap.confirm.failed.heading": "兑换失败",
    "swap.confirm.failed.description": "兑换未能完成。您可以重试或关闭。",
    "swap.confirm.failed.announcement": "兑换失败。",
    "swap.confirm.dropped.heading": "交易超时",
    "swap.confirm.dropped.description": "交易未在截止时间前确认。您可以重新提交或关闭。",
    "swap.confirm.dropped.announcement": "交易超时。",
    "swap.confirm.summary.youPay": "你支付",
    "swap.confirm.summary.youReceive": "你收到",
    "swap.confirm.summary.minReceived": "最少收到",
    "swap.confirm.cta.confirmSwap": "确认兑换",
    "swap.confirm.cta.cancel": "取消",
    "swap.confirm.cta.processing": "处理中\u2026",
    "swap.confirm.cta.done": "完成",
    "swap.confirm.cta.tryAgain": "重试",
    "swap.confirm.cta.dismiss": "关闭",
    "swap.confirm.cta.resubmit": "重新提交",
    "orderbook.title": "订单簿",
    "orderbook.description": "来自所选交易对的实时买单和卖单。",
    "orderbook.button.refresh": "刷新",
    "orderbook.header.price": "价格",
    "orderbook.header.amount": "数量",
    "orderbook.header.total": "总计",
    "orderbook.pairs.loading.title": "加载市场中",
    "orderbook.pairs.loading.description": "获取可用交易对中。",
    "orderbook.pairs.error":
      "⚠️ 市场列表索引器离线。显示本地自适应图表分析工具。",
    "orderbook.loading": "同步订单簿网格...",
    "orderbook.standby": "数据馈送待机模式已激活。",
    "orderbook.highlightedPair": "该交易对当前已在兑换面板中选择",
    "orderbook.bids.title": "买单 ({count})",
    "orderbook.asks.title": "卖单 ({count})",
    "orderbook.noBids": "无可用买单",
    "orderbook.noAsks": "无可用卖单",
    "orderbook.row.bid.aria": "买单价格 {price}，数量 {amount}，总计 {total}",
    "orderbook.row.ask.aria": "卖单价格 {price}，数量 {amount}，总计 {total}",
    "orderbook.depth.title": "市场深度",
    "orderbook.depth.modeLabel": "模式",
    "orderbook.depth.mode.compact": "紧凑 (Canvas)",
    "orderbook.depth.mode.detailed": "详细 (SVG)",
    "orderbook.depth.densityLabel": "密度阈值",
    "orderbook.depth.density.low": "45p (SVG)",
    "orderbook.depth.density.high": "180p (Canvas)",
    "orderbook.depth.perfLabel": "引擎性能",
    "orderbook.depth.fps": "{fps} FPS",
    "orderbook.depth.maxVolume": "最大成交量: {volume} 单位",
    "orderbook.depth.renderCost": "渲染成本",
    "orderbook.depth.memoryProfiler": "内存分析器",
    "orderbook.depth.uiJitter": "UI 抖动边界",
    "orderbook.depth.activeStrategy": "活动策略",
    "orderbook.depth.strategy.canvas": "自适应 Canvas 循环",
    "orderbook.depth.strategy.path": "交互式路径",
    "orderbook.depth.memoryStable": "1.4 MB 稳定",
    "orderbook.depth.uiJitterZero": "0% (无丢帧)",
    "orderbook.depth.cost.canvas": "~0.05ms [O(N)]",
    "orderbook.depth.cost.svg": "~0.22ms [DOM]",
  },
};

const SWAP_TRANSLATIONS: Record<SupportedSwapLocale, SwapTranslations> = {
  ...SWAP_TRANSLATIONS_BASE,
  "de-DE": {
    ...SWAP_TRANSLATIONS_BASE["en-US"],
    "swap.card.title": "Tauschen",
    "swap.pair.youPay": "Du gibst",
    "swap.pair.youReceive": "Du erhältst",
    "swap.cta.reviewSwap": "Swap prüfen",
    "swap.cta.connectWallet": "Wallet verbinden",
    "swap.confirm.confirmed.heading": "Swap bestätigt",
  },
  "fr-FR": {
    ...SWAP_TRANSLATIONS_BASE["en-US"],
    "swap.card.title": "Échanger",
    "swap.pair.youPay": "Vous payez",
    "swap.pair.youReceive": "Vous recevez",
    "swap.cta.reviewSwap": "Vérifier l’échange",
    "swap.cta.connectWallet": "Connecter le wallet",
    "swap.confirm.confirmed.heading": "Échange confirmé",
  },
  "ja-JP": {
    ...SWAP_TRANSLATIONS_BASE["en-US"],
    "swap.card.title": "スワップ",
    "swap.pair.youPay": "支払う",
    "swap.pair.youReceive": "受け取る",
    "swap.cta.reviewSwap": "スワップ内容を確認",
    "swap.cta.connectWallet": "ウォレットを接続",
    "swap.confirm.confirmed.heading": "スワップが確定しました",
  },
};

const SWAP_LOCALE_ALIASES: Record<Locale, SupportedSwapLocale> = {
  "en-US": "en-US",
  "en-GB": "en-US",
  "de-DE": "de-DE",
  "fr-FR": "fr-FR",
  "es-ES": "es-ES",
  "ja-JP": "ja-JP",
  "zh-CN": "zh-CN",
};

function formatMessage(
  template: string,
  variables?: Record<string, string | number>,
) {
  if (!variables) {
    return template;
  }

  return Object.entries(variables).reduce((message, [key, value]) => {
    return message.replaceAll(`{${key}}`, String(value));
  }, template);
}

function getStoredLocale(): Locale | null {
  if (typeof window === "undefined") {
    return null;
  }

  try {
    const raw = window.localStorage.getItem(SETTINGS_STORAGE_KEY);
    if (!raw) {
      return null;
    }

    const parsed = JSON.parse(raw) as { locale?: Locale };
    return parsed.locale ?? null;
  } catch {
    return null;
  }
}

export function resolveSwapLocale(locale?: Locale | null): SupportedSwapLocale {
  const candidate = locale ?? SWAP_FALLBACK_LOCALE;
  return SWAP_LOCALE_ALIASES[candidate] ?? "en-US";
}

export function createSwapTranslator(locale?: Locale | null) {
  const requestedLocale = locale ?? SWAP_FALLBACK_LOCALE;
  const resolvedLocale = resolveSwapLocale(requestedLocale);
  const messages = SWAP_TRANSLATIONS[resolvedLocale] as Record<string, string>;
  const fallbackLocale = resolveSwapLocale(SWAP_FALLBACK_LOCALE);
  const fallbackMessages = SWAP_TRANSLATIONS[fallbackLocale] as Record<
    string,
    string
  >;

  return {
    locale: resolvedLocale,
    fallbackLocale: SWAP_FALLBACK_LOCALE,
    t: (
      key: SwapTranslationKey,
      variables?: Record<string, string | number>,
    ) => {
      const message = messages[key];
      if (message) {
        return formatMessage(message, variables);
      }

      const fallback = fallbackMessages[key];
      if (
        typeof window !== "undefined" &&
        process.env.NODE_ENV !== "production"
      ) {
        console.warn(
          `[swap-i18n] Missing key "${key}" in locale "${resolvedLocale}", falling back to ${SWAP_FALLBACK_LOCALE}.`,
        );
      }
      return formatMessage(fallback ?? key, variables);
    },
  };
}

export function useSwapI18n() {
  const settings = useOptionalSettings();
  const locale =
    settings?.settings.locale ?? getStoredLocale() ?? getUserLocale();

  return createSwapTranslator(locale);
}
