import { useOptionalSettings } from "@/components/providers/settings-provider";
import {
  DEFAULT_LOCALE,
  getUserLocale,
  Locale,
} from "@/lib/formatting";

const SETTINGS_STORAGE_KEY = "stellar_route_settings";

export const OFFRAMP_FALLBACK_LOCALE: Locale = DEFAULT_LOCALE;

type SupportedOfframpLocale =
  | "en-US"
  | "zh-CN"
  | "es-ES"
  | "de-DE"
  | "fr-FR"
  | "ja-JP";

export type OfframpTranslationKey =
  | "offramp.hero.eyebrow"
  | "offramp.hero.title"
  | "offramp.hero.description"
  | "offramp.mode.groupLabel"
  | "offramp.mode.directBadge"
  | "offramp.mode.directTitle"
  | "offramp.mode.directDesc"
  | "offramp.mode.bridgeBadge"
  | "offramp.mode.bridgeTitle"
  | "offramp.mode.bridgeDesc"
  | "offramp.form.amountLabel"
  | "offramp.form.onChain"
  | "offramp.form.bridgeHint"
  | "offramp.form.swapHint"
  | "offramp.form.previewDirect"
  | "offramp.form.previewBridge"
  | "offramp.form.notLiveNotice"
  | "offramp.form.nubanError"
  | "offramp.ready.title"
  | "offramp.ready.directDescription"
  | "offramp.ready.bridgeDescription"
  | "offramp.ready.walletHint"
  | "offramp.rail.title"
  | "offramp.destination.title"
  | "offramp.destination.description"
  | "offramp.destination.liveCorridor"
  | "offramp.destination.bankLabel"
  | "offramp.destination.bankPlaceholder"
  | "offramp.destination.accountNumberLabel"
  | "offramp.destination.accountNumberPlaceholder"
  | "offramp.destination.accountNumberHelp"
  | "offramp.destination.accountNameLabel"
  | "offramp.destination.accountNamePlaceholder"
  | "offramp.summary.emptyTitle"
  | "offramp.summary.emptyDescription"
  | "offramp.summary.receiveLabel"
  | "offramp.summary.rateSubtext"
  | "offramp.summary.youSend"
  | "offramp.summary.path"
  | "offramp.summary.directPath"
  | "offramp.summary.bridgePath"
  | "offramp.summary.previewFee"
  | "offramp.summary.eta"
  | "offramp.source.title"
  | "offramp.source.directDescription"
  | "offramp.source.bridgeDescription"
  | "offramp.source.statusReady"
  | "offramp.source.statusBridge"
  | "offramp.source.statusSwap"
  | "offramp.source.statusSoon";

type OfframpTranslations = Record<OfframpTranslationKey, string>;

const OFFRAMP_TRANSLATIONS: Record<SupportedOfframpLocale, OfframpTranslations> = {
  "en-US": {
    "offramp.hero.eyebrow": "Cash corridor · {flag} Naira first",
    "offramp.hero.title": "Stablecoin to local fiat",
    "offramp.hero.description":
      "Move USDC (or bridge another coin into Stellar USDC) and cash out to Nigerian Naira. Non-custodial on-chain legs; bank payout via partner rails when settlement goes live.",
    "offramp.mode.groupLabel": "Offramp path",
    "offramp.mode.directBadge": "Fastest",
    "offramp.mode.directTitle": "Stellar USDC",
    "offramp.mode.directDesc": "Cash out USDC you already hold on Stellar.",
    "offramp.mode.bridgeBadge": "Any coin",
    "offramp.mode.bridgeTitle": "Bridge + offramp",
    "offramp.mode.bridgeDesc": "Pick any supported coin, bridge to Stellar, then Naira.",
    "offramp.form.amountLabel": "Amount",
    "offramp.form.onChain": "On {chain}",
    "offramp.form.bridgeHint": " · will bridge into Stellar USDC before payout",
    "offramp.form.swapHint": " · swap to USDC on Stellar, then cash out",
    "offramp.form.previewDirect": "Preview Naira payout",
    "offramp.form.previewBridge": "Preview bridge + Naira payout",
    "offramp.form.notLiveNotice":
      "Bank credits are not live yet. This dashboard locks your route and quote shape so partner settlement can plug in without redesigning the flow.",
    "offramp.form.nubanError": "Enter a valid 10-digit NUBAN account number.",
    "offramp.ready.title": "Route ready · ₦{amount} indicative",
    "offramp.ready.directDescription": "Stellar USDC → Nigerian bank.",
    "offramp.ready.bridgeDescription":
      "{symbol} on {chain} → bridge to Stellar USDC → Nigerian bank.",
    "offramp.ready.walletHint":
      "Connect a wallet and complete the on-chain leg when payout partners are enabled on this deployment.",
    "offramp.rail.title": "How it moves",
    "offramp.destination.title": "You receive",
    "offramp.destination.description":
      "First corridor: {flag} {name} to a Nigerian bank account.",
    "offramp.destination.liveCorridor": "Live corridor",
    "offramp.destination.bankLabel": "Bank",
    "offramp.destination.bankPlaceholder": "Select your bank",
    "offramp.destination.accountNumberLabel": "Account number",
    "offramp.destination.accountNumberPlaceholder": "10-digit NUBAN",
    "offramp.destination.accountNumberHelp": "Nigerian bank accounts use a 10-digit NUBAN.",
    "offramp.destination.accountNameLabel": "Account name",
    "offramp.destination.accountNamePlaceholder": "Name on the bank account",
    "offramp.summary.emptyTitle": "Enter an amount to preview Naira",
    "offramp.summary.emptyDescription":
      "Quotes are indicative until the payout partner is connected.",
    "offramp.summary.receiveLabel": "You receive · indicative",
    "offramp.summary.rateSubtext":
      "≈ {netUsdc} USDC after fee · 1 USDC ≈ ₦{rate}",
    "offramp.summary.youSend": "You send",
    "offramp.summary.path": "Path",
    "offramp.summary.directPath": "Direct Stellar USDC",
    "offramp.summary.bridgePath": "Bridge → offramp",
    "offramp.summary.previewFee": "Preview fee ({feePercent}%)",
    "offramp.summary.eta": "ETA",
    "offramp.source.title": "You send",
    "offramp.source.directDescription": "Direct path uses Stellar USDC only.",
    "offramp.source.bridgeDescription":
      "Choose any listed coin — we bridge into Stellar USDC first when needed.",
    "offramp.source.statusReady": "Ready",
    "offramp.source.statusBridge": "Bridge",
    "offramp.source.statusSwap": "Swap",
    "offramp.source.statusSoon": "Soon",
  },
  "es-ES": {
    "offramp.hero.eyebrow": "Corredor de efectivo · {flag} Naira primero",
    "offramp.hero.title": "De stablecoin a moneda local",
    "offramp.hero.description":
      "Transfiere USDC (o haz puente a Stellar USDC) y retira a Naira nigeriano.",
    "offramp.mode.groupLabel": "Ruta de retiro",
    "offramp.mode.directBadge": "Más rápido",
    "offramp.mode.directTitle": "Stellar USDC",
    "offramp.mode.directDesc": "Retira USDC que ya tienes en Stellar.",
    "offramp.mode.bridgeBadge": "Cualquier moneda",
    "offramp.mode.bridgeTitle": "Puente + retiro",
    "offramp.mode.bridgeDesc": "Elige cualquier moneda compatible, haz puente a Stellar y luego a Naira.",
    "offramp.form.amountLabel": "Cantidad",
    "offramp.form.onChain": "En {chain}",
    "offramp.form.bridgeHint": " · se convertirá en Stellar USDC antes del pago",
    "offramp.form.swapHint": " · intercambia a USDC en Stellar, luego retira",
    "offramp.form.previewDirect": "Vista previa de pago en Naira",
    "offramp.form.previewBridge": "Vista previa de puente + pago en Naira",
    "offramp.form.notLiveNotice":
      "Los créditos bancarios aún no están activos en vivo.",
    "offramp.form.nubanError": "Introduce un número de cuenta NUBAN válido de 10 dígitos.",
    "offramp.ready.title": "Ruta lista · ₦{amount} indicativo",
    "offramp.ready.directDescription": "Stellar USDC → Banco nigeriano.",
    "offramp.ready.bridgeDescription":
      "{symbol} en {chain} → puente a Stellar USDC → Banco nigeriano.",
    "offramp.ready.walletHint": "Conecta una billetera cuando los socios de liquidación estén activos.",
    "offramp.rail.title": "Cómo se mueve",
    "offramp.destination.title": "Tú recibes",
    "offramp.destination.description":
      "Primer corredor: {flag} {name} a cuenta bancaria nigeriana.",
    "offramp.destination.liveCorridor": "Corredor en vivo",
    "offramp.destination.bankLabel": "Banco",
    "offramp.destination.bankPlaceholder": "Selecciona tu banco",
    "offramp.destination.accountNumberLabel": "Número de cuenta",
    "offramp.destination.accountNumberPlaceholder": "NUBAN de 10 dígitos",
    "offramp.destination.accountNumberHelp": "Las cuentas bancarias nigerianas usan NUBAN de 10 dígitos.",
    "offramp.destination.accountNameLabel": "Nombre de la cuenta",
    "offramp.destination.accountNamePlaceholder": "Nombre en la cuenta bancaria",
    "offramp.summary.emptyTitle": "Introduce una cantidad para previsualizar Naira",
    "offramp.summary.emptyDescription": "Las cotizaciones son indicativas.",
    "offramp.summary.receiveLabel": "Tú recibes · indicativo",
    "offramp.summary.rateSubtext": "≈ {netUsdc} USDC tras comisión · 1 USDC ≈ ₦{rate}",
    "offramp.summary.youSend": "Tú envías",
    "offramp.summary.path": "Ruta",
    "offramp.summary.directPath": "Directo Stellar USDC",
    "offramp.summary.bridgePath": "Puente → retiro",
    "offramp.summary.previewFee": "Comisión previa ({feePercent}%)",
    "offramp.summary.eta": "Tiempo estimado",
    "offramp.source.title": "Tú envías",
    "offramp.source.directDescription": "La ruta directa usa únicamente Stellar USDC.",
    "offramp.source.bridgeDescription": "Elige cualquier moneda listada.",
    "offramp.source.statusReady": "Listo",
    "offramp.source.statusBridge": "Puente",
    "offramp.source.statusSwap": "Swap",
    "offramp.source.statusSoon": "Pronto",
  },
  "zh-CN": {
    "offramp.hero.eyebrow": "出金通道 · {flag} 奈拉首发",
    "offramp.hero.title": "稳定币兑换本地法币",
    "offramp.hero.description": "转移 USDC（或跨链至 Stellar USDC）并出金至尼日利亚奈拉。",
    "offramp.mode.groupLabel": "出金路径",
    "offramp.mode.directBadge": "极速",
    "offramp.mode.directTitle": "Stellar USDC",
    "offramp.mode.directDesc": "出金你在 Stellar 上已持有的 USDC。",
    "offramp.mode.bridgeBadge": "任意代币",
    "offramp.mode.bridgeTitle": "跨链 + 出金",
    "offramp.mode.bridgeDesc": "选择支持的代币，跨链至 Stellar，然后兑换奈拉。",
    "offramp.form.amountLabel": "金额",
    "offramp.form.onChain": "网络：{chain}",
    "offramp.form.bridgeHint": " · 出金前将跨链至 Stellar USDC",
    "offramp.form.swapHint": " · 在 Stellar 上兑换为 USDC 后出金",
    "offramp.form.previewDirect": "预览奈拉出金",
    "offramp.form.previewBridge": "预览跨链 + 奈拉出金",
    "offramp.form.notLiveNotice": "银行结算通道即将上线。",
    "offramp.form.nubanError": "请输入有效的 10 位 NUBAN 银行账号。",
    "offramp.ready.title": "路径就绪 · ₦{amount}（预估）",
    "offramp.ready.directDescription": "Stellar USDC → 尼日利亚银行。",
    "offramp.ready.bridgeDescription":
      "{symbol}（{chain}）→ 跨链至 Stellar USDC → 尼日利亚银行。",
    "offramp.ready.walletHint": "结算合作方启用后，连接钱包即可完成链上操作。",
    "offramp.rail.title": "流转流程",
    "offramp.destination.title": "你收到",
    "offramp.destination.description": "首发通道：{flag} {name} 至尼日利亚银行账户。",
    "offramp.destination.liveCorridor": "活跃通道",
    "offramp.destination.bankLabel": "银行",
    "offramp.destination.bankPlaceholder": "选择你的银行",
    "offramp.destination.accountNumberLabel": "银行账号",
    "offramp.destination.accountNumberPlaceholder": "10 位 NUBAN 账号",
    "offramp.destination.accountNumberHelp": "尼日利亚银行账户使用 10 位 NUBAN 编码。",
    "offramp.destination.accountNameLabel": "开户姓名",
    "offramp.destination.accountNamePlaceholder": "银行账户姓名",
    "offramp.summary.emptyTitle": "输入金额以预览奈拉",
    "offramp.summary.emptyDescription": "在接入结算伙伴前，报价仅供参考。",
    "offramp.summary.receiveLabel": "你收到 · 预估",
    "offramp.summary.rateSubtext": "扣除费用后 ≈ {netUsdc} USDC · 1 USDC ≈ ₦{rate}",
    "offramp.summary.youSend": "你支付",
    "offramp.summary.path": "路径",
    "offramp.summary.directPath": "直达 Stellar USDC",
    "offramp.summary.bridgePath": "跨链 → 出金",
    "offramp.summary.previewFee": "预估手续费 ({feePercent}%)",
    "offramp.summary.eta": "预估时间",
    "offramp.source.title": "你支付",
    "offramp.source.directDescription": "直达路径仅支持 Stellar USDC。",
    "offramp.source.bridgeDescription": "支持任意列表代币，需要时将自动跨链至 Stellar USDC。",
    "offramp.source.statusReady": "就绪",
    "offramp.source.statusBridge": "跨链",
    "offramp.source.statusSwap": "兑换",
    "offramp.source.statusSoon": "即将推出",
  },
  "de-DE": {
    "offramp.hero.eyebrow": "Auszahlungskorridor · {flag} Naira zuerst",
    "offramp.hero.title": "Stablecoin in lokale Fiat-Währung",
    "offramp.hero.description": "USDC nach Nigerianische Naira auszahlen.",
    "offramp.mode.groupLabel": "Offramp-Pfad",
    "offramp.mode.directBadge": "Am schnellsten",
    "offramp.mode.directTitle": "Stellar USDC",
    "offramp.mode.directDesc": "USDC auszahlen, die Sie bereits auf Stellar halten.",
    "offramp.mode.bridgeBadge": "Beliebiger Coin",
    "offramp.mode.bridgeTitle": "Bridge + Offramp",
    "offramp.mode.bridgeDesc": "Coin wählen, nach Stellar übertragen, dann Naira.",
    "offramp.form.amountLabel": "Betrag",
    "offramp.form.onChain": "Auf {chain}",
    "offramp.form.bridgeHint": " · wird vor Auszahlung nach Stellar USDC gebrückt",
    "offramp.form.swapHint": " · auf Stellar in USDC tauschen, dann auszahlen",
    "offramp.form.previewDirect": "Naira-Auszahlung vorschauen",
    "offramp.form.previewBridge": "Bridge + Naira-Auszahlung vorschauen",
    "offramp.form.notLiveNotice": "Banküberweisungen sind noch nicht live.",
    "offramp.form.nubanError": "Gültige 10-stellige NUBAN-Kontonummer eingeben.",
    "offramp.ready.title": "Route bereit · ₦{amount} indikativ",
    "offramp.ready.directDescription": "Stellar USDC → Nigerianische Bank.",
    "offramp.ready.bridgeDescription":
      "{symbol} auf {chain} → Bridge zu Stellar USDC → Nigerianische Bank.",
    "offramp.ready.walletHint": "Wallet verbinden, sobald Partner live sind.",
    "offramp.rail.title": "Ablauf",
    "offramp.destination.title": "Sie erhalten",
    "offramp.destination.description":
      "Erster Korridor: {flag} {name} auf ein nigerianisches Bankkonto.",
    "offramp.destination.liveCorridor": "Live-Korridor",
    "offramp.destination.bankLabel": "Bank",
    "offramp.destination.bankPlaceholder": "Wählen Sie Ihre Bank",
    "offramp.destination.accountNumberLabel": "Kontonummer",
    "offramp.destination.accountNumberPlaceholder": "10-stellige NUBAN",
    "offramp.destination.accountNumberHelp": "Nigerianische Konten nutzen 10-stellige NUBAN.",
    "offramp.destination.accountNameLabel": "Kontoinhaber",
    "offramp.destination.accountNamePlaceholder": "Name des Kontoinhabers",
    "offramp.summary.emptyTitle": "Betrag eingeben für Naira-Vorschau",
    "offramp.summary.emptyDescription": "Angebote sind indikativ.",
    "offramp.summary.receiveLabel": "Sie erhalten · indikativ",
    "offramp.summary.rateSubtext": "≈ {netUsdc} USDC nach Gebühren · 1 USDC ≈ ₦{rate}",
    "offramp.summary.youSend": "Sie senden",
    "offramp.summary.path": "Pfad",
    "offramp.summary.directPath": "Direkt Stellar USDC",
    "offramp.summary.bridgePath": "Bridge → Offramp",
    "offramp.summary.previewFee": "Vorschau-Gebühr ({feePercent}%)",
    "offramp.summary.eta": "Dauer",
    "offramp.source.title": "Sie senden",
    "offramp.source.directDescription": "Direkter Pfad nutzt nur Stellar USDC.",
    "offramp.source.bridgeDescription": "Beliebigen gelisteten Coin wählen.",
    "offramp.source.statusReady": "Bereit",
    "offramp.source.statusBridge": "Bridge",
    "offramp.source.statusSwap": "Swap",
    "offramp.source.statusSoon": "Bald",
  },
  "fr-FR": {
    "offramp.hero.eyebrow": "Couloir de retrait · {flag} Naira d'abord",
    "offramp.hero.title": "Du stablecoin à la monnaie fiduciaire",
    "offramp.hero.description": "Convertissez vos USDC en Naira nigérian.",
    "offramp.mode.groupLabel": "Option de retrait",
    "offramp.mode.directBadge": "Le plus rapide",
    "offramp.mode.directTitle": "Stellar USDC",
    "offramp.mode.directDesc": "Retirez les USDC déjà détenus sur Stellar.",
    "offramp.mode.bridgeBadge": "Toute devise",
    "offramp.mode.bridgeTitle": "Pont + retrait",
    "offramp.mode.bridgeDesc": "Choisissez une devise, passerelle vers Stellar, puis Naira.",
    "offramp.form.amountLabel": "Montant",
    "offramp.form.onChain": "Sur {chain}",
    "offramp.form.bridgeHint": " · sera ponté en Stellar USDC avant le paiement",
    "offramp.form.swapHint": " · échangez en USDC sur Stellar, puis retirez",
    "offramp.form.previewDirect": "Aperçu du paiement en Naira",
    "offramp.form.previewBridge": "Aperçu pont + paiement en Naira",
    "offramp.form.notLiveNotice": "Les virements bancaires ne sont pas encore actifs.",
    "offramp.form.nubanError": "Entrez un numéro de compte NUBAN valide à 10 chiffres.",
    "offramp.ready.title": "Itinéraire prêt · ₦{amount} indicatif",
    "offramp.ready.directDescription": "Stellar USDC → Banque nigériane.",
    "offramp.ready.bridgeDescription":
      "{symbol} sur {chain} → pont vers Stellar USDC → Banque nigériane.",
    "offramp.ready.walletHint": "Connectez un portefeuille lorsque les partenaires seront actifs.",
    "offramp.rail.title": "Fonctionnement",
    "offramp.destination.title": "Vous recevez",
    "offramp.destination.description":
      "Premier couloir : {flag} {name} vers un compte bancaire nigérian.",
    "offramp.destination.liveCorridor": "Couloir actif",
    "offramp.destination.bankLabel": "Banque",
    "offramp.destination.bankPlaceholder": "Sélectionnez votre banque",
    "offramp.destination.accountNumberLabel": "Numéro de compte",
    "offramp.destination.accountNumberPlaceholder": "NUBAN à 10 chiffres",
    "offramp.destination.accountNumberHelp": "Les comptes nigérians utilisent un NUBAN à 10 chiffres.",
    "offramp.destination.accountNameLabel": "Nom du titulaire",
    "offramp.destination.accountNamePlaceholder": "Nom sur le compte bancaire",
    "offramp.summary.emptyTitle": "Entrez un montant pour afficher l'aperçu",
    "offramp.summary.emptyDescription": "Les taux sont indicatifs.",
    "offramp.summary.receiveLabel": "Vous recevez · indicatif",
    "offramp.summary.rateSubtext": "≈ {netUsdc} USDC après frais · 1 USDC ≈ ₦{rate}",
    "offramp.summary.youSend": "Vous envoyez",
    "offramp.summary.path": "Itinéraire",
    "offramp.summary.directPath": "Direct Stellar USDC",
    "offramp.summary.bridgePath": "Pont → retrait",
    "offramp.summary.previewFee": "Frais estimés ({feePercent}%)",
    "offramp.summary.eta": "Délai estimé",
    "offramp.source.title": "Vous envoyez",
    "offramp.source.directDescription": "Le chemin direct utilise uniquement Stellar USDC.",
    "offramp.source.bridgeDescription": "Choisissez une devise prise en charge.",
    "offramp.source.statusReady": "Prêt",
    "offramp.source.statusBridge": "Pont",
    "offramp.source.statusSwap": "Swap",
    "offramp.source.statusSoon": "Bientôt",
  },
  "ja-JP": {
    "offramp.hero.eyebrow": "出金コリドー · {flag} ナイラ先行",
    "offramp.hero.title": "ステーブルコインを現地法定通貨へ",
    "offramp.hero.description": "USDCをナイジェリアナイラへ出金します。",
    "offramp.mode.groupLabel": "出金ルート",
    "offramp.mode.directBadge": "最速",
    "offramp.mode.directTitle": "Stellar USDC",
    "offramp.mode.directDesc": "Stellarで保有しているUSDCを出金。",
    "offramp.mode.bridgeBadge": "各種コイン",
    "offramp.mode.bridgeTitle": "ブリッジ + 出金",
    "offramp.mode.bridgeDesc": "対応コインを選択し、Stellarへブリッジしてナイラへ。",
    "offramp.form.amountLabel": "金額",
    "offramp.form.onChain": "チェーン: {chain}",
    "offramp.form.bridgeHint": " · 出金前にStellar USDCへブリッジされます",
    "offramp.form.swapHint": " · StellarでUSDCにスワップ後に出金",
    "offramp.form.previewDirect": "ナイラ受取額をプレビュー",
    "offramp.form.previewBridge": "ブリッジ + ナイラ受取額をプレビュー",
    "offramp.form.notLiveNotice": "銀行送金は現在準備中です。",
    "offramp.form.nubanError": "有効な10桁のNUBAN口座番号を入力してください。",
    "offramp.ready.title": "ルート準備完了 · ₦{amount}（概算）",
    "offramp.ready.directDescription": "Stellar USDC → ナイジェリアの銀行口座。",
    "offramp.ready.bridgeDescription":
      "{chain}上の{symbol} → Stellar USDCへブリッジ → ナイジェリアの銀行口座。",
    "offramp.ready.walletHint": "送金機能が稼働したらウォレットを接続して実行してください。",
    "offramp.rail.title": "送金の流れ",
    "offramp.destination.title": "受取内容",
    "offramp.destination.description":
      "第1弾コリドー: {flag} {name}（ナイジェリアの銀行口座宛て）。",
    "offramp.destination.liveCorridor": "稼働コリドー",
    "offramp.destination.bankLabel": "銀行",
    "offramp.destination.bankPlaceholder": "銀行を選択",
    "offramp.destination.accountNumberLabel": "口座番号",
    "offramp.destination.accountNumberPlaceholder": "10桁のNUBAN",
    "offramp.destination.accountNumberHelp": "ナイジェリアの口座番号は10桁のNUBANを使用します。",
    "offramp.destination.accountNameLabel": "口座名義",
    "offramp.destination.accountNamePlaceholder": "口座名義人のお名前",
    "offramp.summary.emptyTitle": "金額を入力してナイラ受取額を表示",
    "offramp.summary.emptyDescription": "レートは概算です。",
    "offramp.summary.receiveLabel": "受取概算額",
    "offramp.summary.rateSubtext": "手数料控除後 ≈ {netUsdc} USDC · 1 USDC ≈ ₦{rate}",
    "offramp.summary.youSend": "支払額",
    "offramp.summary.path": "ルート",
    "offramp.summary.directPath": "Stellar USDC 直接",
    "offramp.summary.bridgePath": "ブリッジ → 出金",
    "offramp.summary.previewFee": "概算手数料 ({feePercent}%)",
    "offramp.summary.eta": "所要時間",
    "offramp.source.title": "支払内容",
    "offramp.source.directDescription": "直接ルートはStellar USDCのみ対応。",
    "offramp.source.bridgeDescription": "一覧のコインを選択してください。",
    "offramp.source.statusReady": "準備完了",
    "offramp.source.statusBridge": "ブリッジ",
    "offramp.source.statusSwap": "スワップ",
    "offramp.source.statusSoon": "近日公開",
  },
};

const OFFRAMP_LOCALE_ALIASES: Record<Locale, SupportedOfframpLocale> = {
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

export function resolveOfframpLocale(locale?: Locale | null): SupportedOfframpLocale {
  const candidate = locale ?? OFFRAMP_FALLBACK_LOCALE;
  return OFFRAMP_LOCALE_ALIASES[candidate] ?? "en-US";
}

export function createOfframpTranslator(locale?: Locale | null) {
  const requestedLocale = locale ?? OFFRAMP_FALLBACK_LOCALE;
  const resolvedLocale = resolveOfframpLocale(requestedLocale);
  const messages = OFFRAMP_TRANSLATIONS[resolvedLocale] as Record<string, string>;
  const fallbackLocale = resolveOfframpLocale(OFFRAMP_FALLBACK_LOCALE);
  const fallbackMessages = OFFRAMP_TRANSLATIONS[fallbackLocale] as Record<
    string,
    string
  >;

  return {
    locale: resolvedLocale,
    fallbackLocale: OFFRAMP_FALLBACK_LOCALE,
    t: (
      key: OfframpTranslationKey,
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
          `[offramp-i18n] Missing key "${key}" in locale "${resolvedLocale}", falling back to ${OFFRAMP_FALLBACK_LOCALE}.`,
        );
      }
      return formatMessage(fallback ?? key, variables);
    },
  };
}

export function useOfframpI18n() {
  const settings = useOptionalSettings();
  const locale =
    settings?.settings.locale ?? getStoredLocale() ?? getUserLocale();

  return createOfframpTranslator(locale);
}
