import { useOptionalSettings } from "@/components/providers/settings-provider";
import {
  DEFAULT_LOCALE,
  getUserLocale,
  Locale,
} from "@/lib/formatting";

const SETTINGS_STORAGE_KEY = "stellar_route_settings";

export const SWAP_FALLBACK_LOCALE: Locale = DEFAULT_LOCALE;

type SupportedSwapLocale = "en-US" | "zh-CN";

type SwapTranslationKey =
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
  | "swap.shortcuts.refreshQuote";

type SwapTranslations = Record<SwapTranslationKey, string>;

const SWAP_TRANSLATIONS: Record<SupportedSwapLocale, SwapTranslations> = {
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
  },
};

const SWAP_LOCALE_ALIASES: Record<Locale, SupportedSwapLocale> = {
  "en-US": "en-US",
  "en-GB": "en-US",
  "de-DE": "en-US",
  "fr-FR": "en-US",
  "es-ES": "en-US",
  "ja-JP": "en-US",
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
  const messages = SWAP_TRANSLATIONS[resolvedLocale];

  return {
    locale: resolvedLocale,
    fallbackLocale: SWAP_FALLBACK_LOCALE,
    t: (
      key: SwapTranslationKey,
      variables?: Record<string, string | number>,
    ) => formatMessage(messages[key], variables),
  };
}

export function useSwapI18n() {
  const settings = useOptionalSettings();
  const locale =
    settings?.settings.locale ?? getStoredLocale() ?? getUserLocale();

  return createSwapTranslator(locale);
}
