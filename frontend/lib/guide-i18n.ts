import { useOptionalSettings } from "@/components/providers/settings-provider";
import {
  DEFAULT_LOCALE,
  getUserLocale,
  Locale,
} from "@/lib/formatting";

const SETTINGS_STORAGE_KEY = "stellar_route_settings";

export const GUIDE_FALLBACK_LOCALE: Locale = DEFAULT_LOCALE;

type SupportedGuideLocale =
  | "en-US"
  | "zh-CN"
  | "es-ES"
  | "de-DE"
  | "fr-FR"
  | "ja-JP";

export type GuideTranslationKey =
  | "guide.meta.title"
  | "guide.meta.description"
  | "guide.header.eyebrow"
  | "guide.header.title"
  | "guide.header.description"
  | "guide.cta.openSwap"
  | "guide.cta.fullGuide"
  | "guide.step.label"
  | "guide.step1.title"
  | "guide.step1.body"
  | "guide.step2.title"
  | "guide.step2.body"
  | "guide.step3.title"
  | "guide.step3.body"
  | "guide.step4.title"
  | "guide.step4.body"
  | "guide.step5.title"
  | "guide.step5.body"
  | "guide.step6.title"
  | "guide.step6.body"
  | "guide.aside.title"
  | "guide.aside.bodyPrefix"
  | "guide.aside.riskDisclosure"
  | "guide.aside.bodySuffix";

type GuideTranslations = Record<GuideTranslationKey, string>;

const GUIDE_TRANSLATIONS: Record<SupportedGuideLocale, GuideTranslations> = {
  "en-US": {
    "guide.meta.title": "First Stellar DEX Swap Guide",
    "guide.meta.description":
      "Step-by-step guide for your first Stellar DEX swap on StellarRoute: wallet, trustline, slippage, and confirm.",
    "guide.header.eyebrow": "User guide",
    "guide.header.title": "Your first live swap",
    "guide.header.description":
      "A short path for traders: wallet → trustline → quote → slippage → confirm. For the full annotated write-up, see the repository guide.",
    "guide.cta.openSwap": "Open swap",
    "guide.cta.fullGuide": "Full guide on GitHub",
    "guide.step.label": "Step {number}",
    "guide.step1.title": "Connect your wallet",
    "guide.step1.body":
      "Use Freighter or xBull. Match the network badge in the footer (Testnet vs Mainnet) before you trade.",
    "guide.step2.title": "Fund and reserve XLM",
    "guide.step2.body":
      "Keep enough XLM for network fees and Stellar base reserves. On testnet, Friendbot can fund a new account.",
    "guide.step3.title": "Add a trustline if needed",
    "guide.step3.body":
      "Non-XLM receive assets usually need a trustline. Approve the trustline transaction in your wallet when prompted.",
    "guide.step4.title": "Pick a pair and enter a small amount",
    "guide.step4.body":
      "Choose pay/receive assets, enter a modest size for your first live swap, and wait for the best-route quote.",
    "guide.step5.title": "Set slippage and review the route",
    "guide.step5.body":
      "Start near 0.5% slippage unless markets are moving quickly. Read high-impact warnings before confirming.",
    "guide.step6.title": "Confirm in your wallet",
    "guide.step6.body":
      "Review amounts in the wallet prompt, sign, then track status in the app. StellarRoute never holds your keys.",
    "guide.aside.title": "Before you confirm",
    "guide.aside.bodyPrefix":
      "Aggregated routes can still slip, fail, or traverse multiple hops. Read the",
    "guide.aside.riskDisclosure": "risk disclosure",
    "guide.aside.bodySuffix":
      "and start with a small amount. Press ? on the swap card anytime for shortcuts and a link back here.",
  },
  "es-ES": {
    "guide.meta.title": "Guía para tu primer intercambio en Stellar DEX",
    "guide.meta.description":
      "Guía paso a paso para tu primer intercambio en StellarRoute: billetera, línea de confianza, deslizamiento y confirmación.",
    "guide.header.eyebrow": "Guía de usuario",
    "guide.header.title": "Tu primer intercambio en vivo",
    "guide.header.description":
      "Un camino corto para traders: billetera → línea de confianza → cotización → deslizamiento → confirmación. Para el texto completo, consulta la guía del repositorio.",
    "guide.cta.openSwap": "Abrir intercambio",
    "guide.cta.fullGuide": "Guía completa en GitHub",
    "guide.step.label": "Paso {number}",
    "guide.step1.title": "Conecta tu billetera",
    "guide.step1.body":
      "Usa Freighter o xBull. Verifica la red en el pie de página (Testnet vs Mainnet) antes de operar.",
    "guide.step2.title": "Fondea y reserva XLM",
    "guide.step2.body":
      "Mantén suficiente XLM para comisiones de red y reservas base de Stellar. En testnet, Friendbot puede fondear una nueva cuenta.",
    "guide.step3.title": "Añade una línea de confianza si es necesario",
    "guide.step3.body":
      "Los activos que no sean XLM suelen necesitar una línea de confianza. Aprueba la transacción en tu billetera cuando se te solicite.",
    "guide.step4.title": "Elige un par e introduce una cantidad pequeña",
    "guide.step4.body":
      "Elige los activos a pagar/recibir, introduce un monto moderado y espera la cotización de mejor ruta.",
    "guide.step5.title": "Ajusta el deslizamiento y revisa la ruta",
    "guide.step5.body":
      "Comienza cerca del 0.5% de deslizamiento a menos que los mercados se muevan rápido. Lee las advertencias de alto impacto.",
    "guide.step6.title": "Confirma en tu billetera",
    "guide.step6.body":
      "Revisa los importes en tu billetera, firma y sigue el estado en la app. StellarRoute nunca custodia tus claves.",
    "guide.aside.title": "Antes de confirmar",
    "guide.aside.bodyPrefix":
      "Las rutas agregadas pueden tener deslizamiento o múltiples saltos. Lee la",
    "guide.aside.riskDisclosure": "divulgación de riesgos",
    "guide.aside.bodySuffix":
      "y comienza con una cantidad pequeña. Presiona ? en la tarjeta de swap para ver atajos.",
  },
  "zh-CN": {
    "guide.meta.title": "Stellar DEX 首次兑换指南",
    "guide.meta.description":
      "在 StellarRoute 进行首次 Stellar DEX 兑换的分步指南：钱包、信任线、滑点与确认。",
    "guide.header.eyebrow": "用户指南",
    "guide.header.title": "你的首次实时兑换",
    "guide.header.description":
      "交易者快速路径：钱包 → 信任线 → 报价 → 滑点 → 确认。如需完整指南，请参阅代码库文档。",
    "guide.cta.openSwap": "进入兑换",
    "guide.cta.fullGuide": "GitHub 完整指南",
    "guide.step.label": "第 {number} 步",
    "guide.step1.title": "连接钱包",
    "guide.step1.body":
      "使用 Freighter 或 xBull。交易前确认页脚的网络标识（测试网或主网）。",
    "guide.step2.title": "充值并保留 XLM",
    "guide.step2.body":
      "保留足够的 XLM 用于网络费用和 Stellar 基础储备。测试网可使用 Friendbot 获取资金。",
    "guide.step3.title": "按需添加信任线",
    "guide.step3.body":
      "非 XLM 接收资产通常需要信任线。在钱包提示时批准信任线交易。",
    "guide.step4.title": "选择交易对并输入少量金额",
    "guide.step4.body":
      "选择支付/接收资产，首次交易建议输入小额，并等待最优路径报价。",
    "guide.step5.title": "设置滑点并检查路由",
    "guide.step5.body":
      "除非市场剧烈波动，建议设置 0.5% 左右的滑点。确认前请仔细阅读高冲击警告。",
    "guide.step6.title": "在钱包中确认",
    "guide.step6.body":
      "在钱包弹窗中核对金额，签名后在应用中跟踪状态。StellarRoute 绝不持有你的私钥。",
    "guide.aside.title": "确认交易之前",
    "guide.aside.bodyPrefix":
      "聚合路由可能存在滑点、失败或多跳。请阅读",
    "guide.aside.riskDisclosure": "风险披露说明",
    "guide.aside.bodySuffix":
      "并从小额开始。在兑换面板随时按 ? 可查看快捷键与返回链接。",
  },
  "de-DE": {
    "guide.meta.title": "Erster Stellar DEX Swap Leitfaden",
    "guide.meta.description":
      "Schritt-für-Schritt-Anleitung für Ihren ersten Stellar DEX Swap auf StellarRoute: Wallet, Trustline, Slippage und Bestätigung.",
    "guide.header.eyebrow": "Benutzerhandbuch",
    "guide.header.title": "Ihr erster Live-Swap",
    "guide.header.description":
      "Schnellstart für Trader: Wallet → Trustline → Angebot → Slippage → Bestätigen.",
    "guide.cta.openSwap": "Swap öffnen",
    "guide.cta.fullGuide": "Vollständige Anleitung auf GitHub",
    "guide.step.label": "Schritt {number}",
    "guide.step1.title": "Wallet verbinden",
    "guide.step1.body":
      "Verwenden Sie Freighter oder xBull. Überprüfen Sie das Netzwerkabzeichen in der Fußzeile.",
    "guide.step2.title": "XLM aufladen und reservieren",
    "guide.step2.body":
      "Halten Sie ausreichend XLM für Netzwerkgebühren und Stellar-Basisreserven bereit.",
    "guide.step3.title": "Trustline hinzufügen (falls erforderlich)",
    "guide.step3.body":
      "Nicht-XLM-Assets benötigen eine Trustline. Bestätigen Sie diese in Ihrer Wallet.",
    "guide.step4.title": "Paar wählen und kleinen Betrag eingeben",
    "guide.step4.body":
      "Wählen Sie Assets aus, geben Sie einen kleinen Betrag ein und warten Sie auf das beste Angebot.",
    "guide.step5.title": "Slippage einstellen und Route prüfen",
    "guide.step5.body":
      "Beginnen Sie bei ca. 0,5% Slippage. Beachten Sie Warnungen vor hohem Price Impact.",
    "guide.step6.title": "In Wallet bestätigen",
    "guide.step6.body":
      "Überprüfen Sie Beträge, signieren Sie und verfolgen Sie den Status. StellarRoute speichert keine Keys.",
    "guide.aside.title": "Vor der Bestätigung",
    "guide.aside.bodyPrefix":
      "Aggregierte Routen können schwanken oder fehlschlagen. Lesen Sie die",
    "guide.aside.riskDisclosure": "Risikohinweise",
    "guide.aside.bodySuffix":
      "und beginnen Sie mit kleinen Beträgen. Drücken Sie jederzeit ? für Tastaturkürzel.",
  },
  "fr-FR": {
    "guide.meta.title": "Guide du premier échange Stellar DEX",
    "guide.meta.description":
      "Guide étape par étape pour votre premier swap Stellar DEX sur StellarRoute.",
    "guide.header.eyebrow": "Guide utilisateur",
    "guide.header.title": "Votre premier swap en direct",
    "guide.header.description":
      "Chemin rapide pour les traders : portefeuille → ligne de confiance → cotation → glissement → confirmation.",
    "guide.cta.openSwap": "Ouvrir l'échange",
    "guide.cta.fullGuide": "Guide complet sur GitHub",
    "guide.step.label": "Étape {number}",
    "guide.step1.title": "Connectez votre portefeuille",
    "guide.step1.body":
      "Utilisez Freighter ou xBull. Vérifiez le réseau dans le pied de page avant d'échanger.",
    "guide.step2.title": "Approvisionnez et réservez des XLM",
    "guide.step2.body":
      "Conservez suffisamment de XLM pour les frais de réseau et les réserves de base Stellar.",
    "guide.step3.title": "Ajoutez une ligne de confiance si nécessaire",
    "guide.step3.body":
      "Les actifs autres que XLM nécessitent une ligne de confiance. Approuvez-la dans votre portefeuille.",
    "guide.step4.title": "Choisissez une paire et entrez un petit montant",
    "guide.step4.body":
      "Choisissez les actifs, entrez un petit montant et attendez la meilleure cotation.",
    "guide.step5.title": "Définissez le glissement et vérifiez l'itinéraire",
    "guide.step5.body":
      "Commencez près de 0,5% de glissement. Lisez les avertissements d'impact sur les prix.",
    "guide.step6.title": "Confirmez dans votre portefeuille",
    "guide.step6.body":
      "Vérifiez les montants, signez et suivez l'état. StellarRoute ne détient jamais vos clés.",
    "guide.aside.title": "Avant de confirmer",
    "guide.aside.bodyPrefix":
      "Les itinéraires agrégés peuvent glisser ou échouer. Lisez la",
    "guide.aside.riskDisclosure": "divulgation des risques",
    "guide.aside.bodySuffix":
      "et commencez avec un petit montant. Appuyez sur ? pour les raccourcis.",
  },
  "ja-JP": {
    "guide.meta.title": "Stellar DEX 初回スワップガイド",
    "guide.meta.description":
      "StellarRouteでの初回Stellar DEXスワップのステップバイステップガイド。",
    "guide.header.eyebrow": "ユーザーガイド",
    "guide.header.title": "初めてのライブスワップ",
    "guide.header.description":
      "トレーダー向けクイックガイド：ウォレット → トラストライン → レート見積もり → スリッページ → 確定。",
    "guide.cta.openSwap": "スワップを開く",
    "guide.cta.fullGuide": "GitHubの完全ガイド",
    "guide.step.label": "ステップ {number}",
    "guide.step1.title": "ウォレットを接続",
    "guide.step1.body":
      "FreighterまたはxBullを使用します。取引前にフッターのネットワーク表示を確認してください。",
    "guide.step2.title": "XLMの入金と準備金確保",
    "guide.step2.body":
      "ネットワーク手数料とStellar基本準備金のために十分なXLMを確保してください。",
    "guide.step3.title": "必要に応じてトラストラインを追加",
    "guide.step3.body":
      "XLM以外のアセットを受け取るにはトラストラインが必要です。ウォレットで承認してください。",
    "guide.step4.title": "ペアを選択し少額を入力",
    "guide.step4.body":
      "支払/受取アセットを選択し、少額を入力して最適ルートの見積もりを待ちます。",
    "guide.step5.title": "スリッページを設定しルートを確認",
    "guide.step5.body":
      "最初は約0.5%のスリッページから開始してください。価格影響の警告を確認してください。",
    "guide.step6.title": "ウォレットで確定",
    "guide.step6.body":
      "金額を確認して署名し、アプリでステータスを確認します。鍵はご自身で安全に管理されます。",
    "guide.aside.title": "確定する前に",
    "guide.aside.bodyPrefix":
      "ルートにはスリッページや複数ホップのリスクがあります。",
    "guide.aside.riskDisclosure": "リスク開示",
    "guide.aside.bodySuffix":
      "をご確認の上、少額からお試しください。スワップカードで ? を押すとショートカットを表示します。",
  },
};

const GUIDE_LOCALE_ALIASES: Record<Locale, SupportedGuideLocale> = {
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

export function resolveGuideLocale(locale?: Locale | null): SupportedGuideLocale {
  const candidate = locale ?? GUIDE_FALLBACK_LOCALE;
  return GUIDE_LOCALE_ALIASES[candidate] ?? "en-US";
}

export function createGuideTranslator(locale?: Locale | null) {
  const requestedLocale = locale ?? GUIDE_FALLBACK_LOCALE;
  const resolvedLocale = resolveGuideLocale(requestedLocale);
  const messages = GUIDE_TRANSLATIONS[resolvedLocale] as Record<string, string>;
  const fallbackLocale = resolveGuideLocale(GUIDE_FALLBACK_LOCALE);
  const fallbackMessages = GUIDE_TRANSLATIONS[fallbackLocale] as Record<
    string,
    string
  >;

  return {
    locale: resolvedLocale,
    fallbackLocale: GUIDE_FALLBACK_LOCALE,
    t: (
      key: GuideTranslationKey,
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
          `[guide-i18n] Missing key "${key}" in locale "${resolvedLocale}", falling back to ${GUIDE_FALLBACK_LOCALE}.`,
        );
      }
      return formatMessage(fallback ?? key, variables);
    },
  };
}

export function useGuideI18n() {
  const settings = useOptionalSettings();
  const locale =
    settings?.settings.locale ?? getStoredLocale() ?? getUserLocale();

  return createGuideTranslator(locale);
}
