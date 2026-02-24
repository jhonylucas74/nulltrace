import { useState, useEffect, useMemo, useCallback } from "react";
import {
  ArrowLeft,
  Check,
  LayoutDashboard,
  Receipt,
  Send,
  Key,
  ArrowLeftRight,
  Copy,
  Settings2,
  Search,
  Calendar,
  CreditCard,
  Eye,
  EyeOff,
  Plus,
  Trash2,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { getRate } from "../lib/walletConversion";
import { useWallet, parseAmount, applyAmountMask, formatAmount, type WalletTx } from "../contexts/WalletContext";
import styles from "./WalletApp.module.css";

/** Static currency metadata. Amounts come from the real backend. */
const CURRENCIES = [
  { symbol: "USD", currency: "US Dollar" },
  { symbol: "BTC", currency: "Bitcoin" },
  { symbol: "ETH", currency: "Ethereum" },
  { symbol: "SOL", currency: "Solana" },
];

type StatementPeriod = "today" | "7d" | "30d" | "all";

function filterStatement(
  transactions: WalletTx[],
  period: StatementPeriod,
  searchQuery: string
): WalletTx[] {
  const now = Date.now();
  const oneDay = 24 * 60 * 60 * 1000;
  let start = 0;
  if (period === "today") {
    const today = new Date();
    today.setHours(0, 0, 0, 0);
    start = today.getTime();
  } else if (period === "7d") {
    start = now - 7 * oneDay;
  } else if (period === "30d") {
    start = now - 30 * oneDay;
  }

  const q = searchQuery.trim().toLowerCase();
  return transactions.filter((tx) => {
    if (period !== "all" && tx.created_at_ms < start) return false;
    if (!q) return true;
    return (
      tx.description.toLowerCase().includes(q) ||
      tx.currency.toLowerCase().includes(q) ||
      tx.tx_type.toLowerCase().includes(q)
    );
  });
}

function formatCardNumber(number: string): string {
  return number.replace(/(.{4})/g, "$1 ").trim();
}

const STORAGE_KEY = "wallet-visible-currencies";

function loadVisibleSymbols(): Set<string> {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) {
      const arr = JSON.parse(raw) as string[];
      if (Array.isArray(arr)) return new Set(arr);
    }
  } catch { /* ignore */ }
  return new Set(CURRENCIES.map((c) => c.symbol));
}

function saveVisibleSymbols(symbols: Set<string>) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify([...symbols]));
}

type Section = "overview" | "statement" | "transfer" | "keys" | "card" | "convert";
type View = "main" | "select";

function WalletContent() {
  const { t } = useTranslation("wallet");
  const wallet = useWallet();
  const [visibleSymbols, setVisibleSymbols] = useState<Set<string>>(loadVisibleSymbols);
  const [section, setSection] = useState<Section>("overview");
  const [view, setView] = useState<View>("main");
  const [draftSelection, setDraftSelection] = useState<Set<string>>(visibleSymbols);
  const [statementPeriod, setStatementPeriod] = useState<StatementPeriod>("all");
  const [statementSearch, setStatementSearch] = useState("");

  useEffect(() => {
    saveVisibleSymbols(visibleSymbols);
  }, [visibleSymbols]);

  // Refresh transactions when filter changes
  useEffect(() => {
    wallet.fetchTransactions(statementPeriod);
  }, [statementPeriod]); // eslint-disable-line react-hooks/exhaustive-deps

  const openSelectPage = () => {
    setDraftSelection(new Set(visibleSymbols));
    setView("select");
  };

  const toggleDraft = (symbol: string) => {
    setDraftSelection((prev) => {
      const next = new Set(prev);
      if (next.has(symbol)) {
        if (next.size <= 1) return prev;
        next.delete(symbol);
      } else next.add(symbol);
      return next;
    });
  };

  const confirmSelection = () => {
    setVisibleSymbols(new Set(draftSelection));
    setView("main");
  };

  const visibleCurrencies = useMemo(
    () => CURRENCIES.filter((c) => visibleSymbols.has(c.symbol)),
    [visibleSymbols]
  );

  if (view === "select") {
    return (
      <div className={styles.app}>
        <div className={styles.selectPage}>
          <div className={styles.selectHeader}>
            <button
              type="button"
              className={styles.backBtn}
              onClick={() => setView("main")}
              title="Back"
              aria-label="Back"
            >
              <ArrowLeft size={20} />
            </button>
            <h2 className={styles.selectTitle}>Select currencies</h2>
          </div>
          <p className={styles.selectSubtitle}>
            Choose which currencies to show on your wallet. At least one must be selected.
          </p>
          <ul className={styles.selectList}>
            {CURRENCIES.map((c) => {
              const isChecked = draftSelection.has(c.symbol);
              const isOnlyOne = isChecked && draftSelection.size === 1;
              return (
                <li key={c.symbol}>
                  <button
                    type="button"
                    className={styles.selectItem}
                    onClick={() => !isOnlyOne && toggleDraft(c.symbol)}
                    aria-pressed={isChecked}
                    disabled={isOnlyOne}
                  >
                    <span className={styles.selectItemCheck}>
                      {isChecked ? <Check size={16} /> : null}
                    </span>
                    <span className={styles.selectItemSymbol}>{c.symbol}</span>
                    <span className={styles.selectItemCurrency}>{c.currency}</span>
                  </button>
                </li>
              );
            })}
          </ul>
          <div className={styles.selectFooter}>
            <button type="button" className={styles.doneBtn} onClick={confirmSelection}>
              Done
            </button>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className={styles.appWithSidebar}>
      <aside className={styles.sidebar}>
        <div className={styles.sidebarSection}>Wallet</div>
        <button
          type="button"
          className={`${styles.navItem} ${section === "overview" ? styles.navItemActive : ""}`}
          onClick={() => setSection("overview")}
        >
          <span className={styles.navIcon}><LayoutDashboard size={18} /></span>
          {t("nav_overview")}
        </button>
        <button
          type="button"
          className={`${styles.navItem} ${section === "statement" ? styles.navItemActive : ""}`}
          onClick={() => setSection("statement")}
        >
          <span className={styles.navIcon}><Receipt size={18} /></span>
          {t("nav_statement")}
        </button>
        <button
          type="button"
          className={`${styles.navItem} ${section === "transfer" ? styles.navItemActive : ""}`}
          onClick={() => setSection("transfer")}
        >
          <span className={styles.navIcon}><Send size={18} /></span>
          {t("nav_transfer")}
        </button>
        <button
          type="button"
          className={`${styles.navItem} ${section === "keys" ? styles.navItemActive : ""}`}
          onClick={() => setSection("keys")}
        >
          <span className={styles.navIcon}><Key size={18} /></span>
          {t("nav_keys")}
        </button>
        <button
          type="button"
          className={`${styles.navItem} ${section === "card" ? styles.navItemActive : ""}`}
          onClick={() => setSection("card")}
        >
          <span className={styles.navIcon}><CreditCard size={18} /></span>
          {t("nav_card")}
        </button>
        <button
          type="button"
          className={`${styles.navItem} ${section === "convert" ? styles.navItemActive : ""}`}
          onClick={() => setSection("convert")}
        >
          <span className={styles.navIcon}><ArrowLeftRight size={18} /></span>
          {t("nav_convert")}
        </button>
      </aside>
      <main className={styles.main}>
        {section === "overview" && (
          <>
            <div className={styles.header}>
              <div>
                <h2 className={styles.mainTitle}>{t("overview_title")}</h2>
                <p className={styles.mainSubtitle}>{t("overview_subtitle")}</p>
              </div>
              <button
                type="button"
                className={styles.configBtn}
                onClick={openSelectPage}
                title={t("overview_manage_currencies")}
              >
                <Settings2 size={14} />
                {t("overview_manage_currencies")}
              </button>
            </div>
            <div className={styles.cards}>
              {visibleCurrencies.map((item) => (
                <div key={item.symbol} className={styles.card}>
                  <div className={styles.cardHeader}>
                    <span className={styles.symbol}>{item.symbol}</span>
                    <span className={styles.currency}>{item.currency}</span>
                    {item.symbol === "USD" && (
                      <span className={styles.cardBadge} title="Managed by Fkebank">Fkebank</span>
                    )}
                  </div>
                  <div className={styles.amount}>
                    {wallet.getFormattedBalance(item.symbol)}
                  </div>
                  {item.symbol === "USD" && (
                      <p className={styles.cardUsdNote}>
                      {t("overview_usd_note")}
                    </p>
                  )}
                  {item.symbol !== "USD" && (
                    <p className={styles.cardUsdEquivalent}>
                      ≈{" "}
                      {(
                        ((wallet.balances[item.symbol] ?? 0) / 100) * getRate(item.symbol, "USD")
                      ).toLocaleString("en-US", {
                        minimumFractionDigits: 2,
                        maximumFractionDigits: 2,
                      })}{" "}
                      USD
                    </p>
                  )}
                </div>
              ))}
            </div>
            <div className={styles.overviewCardDebt}>
              <div className={styles.overviewCardDebtRow}>
                <span className={styles.overviewCardDebtLabel}>{t("overview_card_debt_label")}</span>
                <span className={styles.overviewCardDebtValue}>
                  {(wallet.cardDebt / 100).toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 })} USD
                </span>
              </div>
              {wallet.cardLimit > 0 && (
                <div className={styles.overviewCardDebtProgressWrap}>
                  <div className={styles.cardLimitProgressTrack}>
                    <div
                      className={`${styles.cardLimitProgressFill} ${Math.round((wallet.cardDebt / wallet.cardLimit) * 100) >= 80 ? styles.cardLimitProgressFillHigh : ""}`}
                      style={{
                        width: `${Math.min(100, (wallet.cardDebt / wallet.cardLimit) * 100)}%`,
                      }}
                      role="progressbar"
                      aria-valuenow={Math.round((wallet.cardDebt / wallet.cardLimit) * 100)}
                      aria-valuemin={0}
                      aria-valuemax={100}
                      aria-label="Card limit usage"
                    />
                  </div>
                  <div className={styles.cardLimitProgressLabel}>
                    {(wallet.cardDebt / 100).toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 })} of{" "}
                    {(wallet.cardLimit / 100).toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 })} USD
                  </div>
                </div>
              )}
            </div>
          </>
        )}

        {section === "statement" && (
          <StatementSection
            wallet={wallet}
            statementPeriod={statementPeriod}
            setStatementPeriod={setStatementPeriod}
            statementSearch={statementSearch}
            setStatementSearch={setStatementSearch}
          />
        )}

        {section === "transfer" && (
          <TransferSection wallet={wallet} />
        )}

        {section === "keys" && (
          <KeysSection wallet={wallet} />
        )}

        {section === "card" && (
          <CardSection wallet={wallet} />
        )}

        {section === "convert" && (
          <ConvertSection wallet={wallet} />
        )}
      </main>
    </div>
  );
}

function StatementSection({
  wallet,
  statementPeriod,
  setStatementPeriod,
  statementSearch,
  setStatementSearch,
}: {
  wallet: ReturnType<typeof useWallet>;
  statementPeriod: StatementPeriod;
  setStatementPeriod: (p: StatementPeriod) => void;
  statementSearch: string;
  setStatementSearch: (s: string) => void;
}) {
  const filtered = filterStatement(wallet.transactions, statementPeriod, statementSearch);

  return (
    <>
      <h2 className={styles.mainTitle}>Statement</h2>
      <p className={styles.mainSubtitle}>Recent transactions. Filter by period or search.</p>
      <div className={styles.statementToolbar}>
        <div className={styles.statementPeriodRow}>
          <span className={styles.statementPeriodLabel} aria-hidden="true">
            <Calendar size={14} />
          </span>
          {(["today", "7d", "30d", "all"] as const).map((p) => (
            <button
              key={p}
              type="button"
              className={`${styles.statementPeriodBtn} ${statementPeriod === p ? styles.statementPeriodBtnActive : ""}`}
              onClick={() => setStatementPeriod(p)}
            >
              {p === "today" ? "Today" : p === "7d" ? "7 days" : p === "30d" ? "30 days" : "All"}
            </button>
          ))}
        </div>
        <div className={styles.statementSearchWrap}>
          <Search size={16} className={styles.statementSearchIcon} aria-hidden="true" />
          <input
            type="search"
            className={styles.statementSearchInput}
            placeholder="Search description, currency…"
            value={statementSearch}
            onChange={(e) => setStatementSearch(e.target.value)}
            aria-label="Search transactions"
          />
        </div>
      </div>
      <ul className={styles.statementList}>
        {filtered.map((tx) => {
          const isCredit = tx.tx_type === "credit" || tx.tx_type === "transfer_in";
          const amountClass = isCredit ? styles.statementAmountCredit : styles.statementAmountDebit;
          const sign = isCredit ? "+" : "-";
          const displayAmount = `${sign}${formatAmount(Math.abs(tx.amount), tx.currency)}`;
          const dateStr = new Date(tx.created_at_ms).toLocaleString("en-US", {
            month: "short", day: "numeric", hour: "2-digit", minute: "2-digit",
          });
          return (
            <li key={tx.id} className={styles.statementRow}>
              <span className={styles.statementDate}>{dateStr}</span>
              <span className={styles.statementDesc}>{tx.description || tx.tx_type}</span>
              <span className={`${styles.statementAmount} ${amountClass}`}>
                {displayAmount} {tx.currency}
              </span>
            </li>
          );
        })}
      </ul>
      {filtered.length === 0 && (
        <p className={styles.statementEmpty}>No transactions match the filter.</p>
      )}
    </>
  );
}

function TransferSection({ wallet }: { wallet: ReturnType<typeof useWallet> }) {
  const { t } = useTranslation("wallet");
  const [recipientKey, setRecipientKey] = useState("");
  const [amountStr, setAmountStr] = useState("");
  const [currency, setCurrency] = useState("USD");
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<{ type: "success" | "error"; text: string } | null>(null);
  const [keyResolution, setKeyResolution] = useState<{
    is_valid: boolean;
    is_usd: boolean;
    account_holder_name: string;
    target_currency: string;
  } | null>(null);

  // Debounced key validation: resolve after user stops typing
  useEffect(() => {
    const key = recipientKey.trim();
    if (!key) {
      setKeyResolution(null);
      return;
    }
    const timer = setTimeout(() => {
      wallet.resolveTransferKey(key).then(setKeyResolution);
    }, 400);
    return () => clearTimeout(timer);
  }, [recipientKey, wallet]);

  const balanceCents = wallet.balances[currency] ?? 0;
  const displayAmount = parseAmount(amountStr);
  const amountCents = Math.round(displayAmount * 100);
  const exceedsBalance = displayAmount > 0 && amountCents > balanceCents;
  const zeroBalance = balanceCents === 0;
  const keyValid = recipientKey.trim() === "" || (keyResolution != null && keyResolution.is_valid);
  const canSubmit =
    !busy &&
    !zeroBalance &&
    !exceedsBalance &&
    recipientKey.trim().length > 0 &&
    displayAmount > 0 &&
    keyValid;

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setMessage(null);
    const key = recipientKey.trim();
    if (!key) {
      setMessage({ type: "error", text: t("transfer_enter_key") });
      return;
    }
    if (displayAmount <= 0) {
      setMessage({ type: "error", text: t("transfer_enter_valid_amount") });
      return;
    }
    if (exceedsBalance) {
      setMessage({ type: "error", text: t("transfer_exceeds_balance") });
      return;
    }
    if (keyResolution != null && !keyResolution.is_valid) {
      setMessage({ type: "error", text: t("transfer_invalid_key") });
      return;
    }
    setBusy(true);
    const err = await wallet.transfer(currency, amountCents, key, keyResolution?.target_currency);
    setBusy(false);
    if (err === null) {
      setMessage({ type: "success", text: t("transfer_success") });
      setRecipientKey("");
      setAmountStr("");
    } else {
      console.error("[Wallet Transfer] failed – raw error for investigation:", {
        rawError: err,
        currency,
        amountCents,
        targetCurrency: keyResolution?.target_currency ?? "(same)",
        recipientKeyLength: key.length,
        recipientKeyPrefix: key.slice(0, 30) + (key.length > 30 ? "…" : ""),
      });
      const userMsg = err.includes("InsufficientBalance")
        ? t("transfer_insufficient_balance")
        : err.includes("UNAUTHENTICATED")
          ? t("transfer_session_expired")
          : err === "CONVERTED_AMOUNT_TOO_SMALL" || err.includes("Converted amount is zero") || err.includes("too small")
            ? t("transfer_converted_amount_too_small")
            : t("transfer_failed");
      setMessage({ type: "error", text: userMsg });
    }
  };

  return (
    <>
      <h2 className={styles.mainTitle}>{t("transfer_title")}</h2>
      <p className={styles.mainSubtitle}>{t("transfer_subtitle")}</p>
      <form className={styles.form} onSubmit={handleSubmit}>
        <div className={styles.formGroup}>
          <label className={styles.formLabel} htmlFor="transfer-key">{t("transfer_recipient_label")}</label>
          <input
            id="transfer-key"
            type="text"
            className={`${styles.formInput} ${keyResolution?.is_valid ? styles.formInputValid : ""}`}
            value={recipientKey}
            onChange={(e) => setRecipientKey(e.target.value)}
            placeholder={t("transfer_recipient_placeholder")}
            disabled={busy}
          />
          {keyResolution?.is_valid && keyResolution.is_usd && keyResolution.account_holder_name && (
            <p className={styles.transferAccountHolder}>
              {t("transfer_account_holder")}: {keyResolution.account_holder_name}
            </p>
          )}
          {keyResolution?.is_valid && keyResolution.target_currency && keyResolution.target_currency !== currency && (
            <p className={styles.transferAccountHolder} aria-live="polite">
              {t("transfer_cross_currency_hint", { from: currency, to: keyResolution.target_currency })}
            </p>
          )}
        </div>
        <div className={styles.formGroup}>
          <label className={styles.formLabel} htmlFor="transfer-amount">{t("transfer_amount_label")}</label>
          <input
            id="transfer-amount"
            type="text"
            inputMode="decimal"
            autoComplete="off"
            className={styles.formInput}
            value={amountStr}
            onChange={(e) => setAmountStr(applyAmountMask(e.target.value, currency === "USD" ? 2 : 8))}
            placeholder={t("transfer_amount_placeholder")}
            disabled={busy || zeroBalance}
            aria-invalid={exceedsBalance}
          />
        </div>
        <div className={styles.formGroup}>
          <label className={styles.formLabel} htmlFor="transfer-currency">{t("transfer_currency_label")}</label>
          <select
            id="transfer-currency"
            className={styles.formSelect}
            value={currency}
            onChange={(e) => setCurrency(e.target.value)}
            disabled={busy}
          >
            {CURRENCIES.map((c) => (
              <option key={c.symbol} value={c.symbol}>{c.symbol}</option>
            ))}
          </select>
        </div>
        {exceedsBalance && (
          <p className={`${styles.formMessage} ${styles.formMessageError}`} role="alert">
            {t("transfer_exceeds_balance")}
          </p>
        )}
        {message && !exceedsBalance && (
          <p className={`${styles.formMessage} ${message.type === "success" ? styles.formMessageSuccess : styles.formMessageError}`}>
            {message.text}
          </p>
        )}
        <button type="submit" className={styles.submitBtn} disabled={!canSubmit}>
          {busy ? t("transfer_sending") : t("transfer_send")}
        </button>
      </form>
      <div className={`${styles.convertBalanceSummary} ${styles.transferBalanceSummary}`}>
        <div className={styles.convertBalanceSummaryTitle}>{t("transfer_balance_summary_title")}</div>
        <div className={styles.convertBalanceSummaryRow}>
          {CURRENCIES.map((c) => (
            <span key={c.symbol} className={styles.convertBalanceSummaryItem}>
              <span className={styles.convertBalanceSummarySymbol}>{c.symbol}</span>
              <span className={styles.convertBalanceSummaryAmount}>{wallet.getFormattedBalance(c.symbol)}</span>
            </span>
          ))}
        </div>
      </div>
    </>
  );
}

function KeysSection({ wallet }: { wallet: ReturnType<typeof useWallet> }) {
  const { t } = useTranslation("wallet");
  const [copied, setCopied] = useState<string | null>(null);

  const copyToClipboard = async (text: string, id: string) => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(id);
      setTimeout(() => setCopied(null), 2000);
    } catch { /* ignore */ }
  };

  if (wallet.keys.length === 0) {
    return (
      <>
        <h2 className={styles.mainTitle}>Keys</h2>
        <p className={styles.mainSubtitle}>Your receive keys and addresses.</p>
        <p className={styles.statementEmpty}>Loading keys…</p>
      </>
    );
  }

  return (
    <>
      <h2 className={styles.mainTitle}>Keys</h2>
      <p className={styles.mainSubtitle}>Your receive keys and addresses per currency.</p>

      {wallet.keys.map((k) => (
        <div key={k.currency} className={styles.keyBlock}>
          <div className={styles.keyBlockTitle}>
            {k.currency} receive {k.currency === "USD" ? "key (Fkebank)" : "address"}
          </div>
          <div className={styles.keyValue}>{k.key_address}</div>
          {k.currency === "USD" && (
            <p className={styles.keyExplanation}>
              Your USD balance is managed by Fkebank. Use this key to receive USD transfers.
            </p>
          )}
          <div className={styles.keyCopyWrap}>
            <button
              type="button"
              className={styles.keyCopyBtn}
              onClick={() => copyToClipboard(k.key_address, k.currency)}
              aria-label={`Copy ${k.currency} key`}
            >
              <Copy size={14} />
              {copied === k.currency ? "Copied" : t("keys_copy")}
            </button>
          </div>
        </div>
      ))}
    </>
  );
}

type CardTab = "cards" | "statement";

function CardSection({ wallet }: { wallet: ReturnType<typeof useWallet> }) {
  const { t } = useTranslation("wallet");
  const [cardTab, setCardTab] = useState<CardTab>("statement");
  const [cvvRevealed, setCvvRevealed] = useState<string | null>(null);
  const [copiedCardId, setCopiedCardId] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);

  const firstCard = wallet.cards[0] ?? null;

  // Fetch statement for the first card when tab mounts
  useEffect(() => {
    if (firstCard) {
      wallet.fetchCardStatement(firstCard.id);
      wallet.fetchCardTransactions(firstCard.id, "all");
    }
  }, [firstCard?.id]); // eslint-disable-line react-hooks/exhaustive-deps

  const cardDebtDisplay = firstCard ? firstCard.current_debt / 100 : 0;
  const cardLimitDisplay = firstCard ? firstCard.credit_limit / 100 : 0;
  const usageRatio = cardLimitDisplay > 0 ? Math.min(1, cardDebtDisplay / cardLimitDisplay) : 0;
  const usagePercent = Math.round(usageRatio * 100);

  const dueDate = wallet.cardStatement ? new Date(wallet.cardStatement.due_date_ms) : null;
  const dueDateStr = dueDate
    ? dueDate.toLocaleDateString("en-US", { weekday: "short", month: "short", day: "numeric", year: "numeric" })
    : "—";

  const copyCardNumber = async (card: (typeof wallet.cards)[0]) => {
    try {
      await navigator.clipboard.writeText(card.number_full.replace(/\s/g, ""));
      setCopiedCardId(card.id);
      setTimeout(() => setCopiedCardId(null), 2000);
    } catch { /* ignore */ }
  };

  const toggleCvv = (cardId: string) => {
    const next = cvvRevealed === cardId ? null : cardId;
    setCvvRevealed(next);
    if (next === cardId) {
      setTimeout(() => setCvvRevealed((c) => (c === cardId ? null : c)), 5000);
    }
  };

  const handleCreateCard = async () => {
    setCreating(true);
    await wallet.createCard("Virtual " + (wallet.cards.length + 1), 0);
    setCreating(false);
  };

  const handleDeleteCard = async (cardId: string) => {
    await wallet.deleteCard(cardId);
  };

  const handlePayBill = async () => {
    if (!firstCard) return;
    await wallet.payBill(firstCard.id);
  };

  return (
    <>
      <h2 className={styles.mainTitle}>Credit card</h2>
      <p className={styles.mainSubtitle}>
        Fkebank credit card linked to your USD balance. The invoice is charged automatically every 7 days.
      </p>
      <div className={styles.cardTabs}>
        <button
          type="button"
          className={`${styles.cardTabBtn} ${cardTab === "statement" ? styles.cardTabBtnActive : ""}`}
          onClick={() => setCardTab("statement")}
        >
          Statement
        </button>
        <button
          type="button"
          className={`${styles.cardTabBtn} ${cardTab === "cards" ? styles.cardTabBtnActive : ""}`}
          onClick={() => setCardTab("cards")}
        >
          Cards
        </button>
      </div>

      {cardTab === "cards" && (
        <>
          <div className={styles.virtualCardList}>
            {wallet.cards.map((card) => (
              <div key={card.id} className={styles.virtualCard}>
                <div className={styles.virtualCardHeader}>
                  <CreditCard size={18} className={styles.virtualCardIcon} aria-hidden="true" />
                  <span className={styles.virtualCardLabel}>{card.label || "Card"}</span>
                  <button
                    type="button"
                    className={styles.virtualCardDeleteBtn}
                    onClick={() => handleDeleteCard(card.id)}
                    aria-label="Remove card"
                    title="Remove card"
                  >
                    <Trash2 size={16} />
                  </button>
                </div>
                <div className={styles.virtualCardNumberRow}>
                  <span className={styles.virtualCardNumber}>{formatCardNumber(card.number_full)}</span>
                  <button
                    type="button"
                    className={styles.virtualCardCopyBtn}
                    onClick={() => copyCardNumber(card)}
                    aria-label="Copy card number"
                    title="Copy number"
                  >
                    <Copy size={14} />
                    {copiedCardId === card.id ? "Copied" : t("keys_copy")}
                  </button>
                </div>
                <div className={styles.virtualCardMeta}>
                  <span>{card.holder_name}</span>
                  <span className={styles.virtualCardValidity}>
                    Valid thru {String(card.expiry_month).padStart(2, "0")}/{card.expiry_year}
                  </span>
                </div>
                <div className={styles.virtualCardCvvRow}>
                  <span className={styles.virtualCardCvvLabel}>Security code (CVV)</span>
                  <span className={styles.virtualCardCvvValue}>
                    {cvvRevealed === card.id ? card.cvv : "•••"}
                  </span>
                  <button
                    type="button"
                    className={styles.virtualCardCvvBtn}
                    onClick={() => toggleCvv(card.id)}
                    aria-label={cvvRevealed === card.id ? "Hide security code" : "Show security code"}
                  >
                    {cvvRevealed === card.id ? <EyeOff size={14} /> : <Eye size={14} />}
                  </button>
                </div>
              </div>
            ))}
          </div>
          <button type="button" className={styles.addCardBtn} onClick={handleCreateCard} disabled={creating}>
            <Plus size={18} />
            {creating ? "Creating…" : t("card_new_virtual")}
          </button>
        </>
      )}

      {cardTab === "statement" && (
        <div className={styles.cardStatementSection}>
          <div className={styles.cardSummaryBlock}>
            <div className={styles.cardSummaryRow}>
              <span className={styles.cardSummaryLabel}>{t("card_debt")}</span>
              <span className={styles.cardSummaryValue}>
                {cardDebtDisplay.toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 })} USD
              </span>
            </div>
            <div className={styles.cardSummaryRow}>
              <span className={styles.cardSummaryLabel}>{t("card_limit")}</span>
              <span className={styles.cardSummaryValue}>
                {cardLimitDisplay.toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 })} USD
              </span>
            </div>
            <div className={styles.cardLimitProgressWrap}>
              <div className={styles.cardLimitProgressTrack}>
                <div
                  className={`${styles.cardLimitProgressFill} ${usagePercent >= 80 ? styles.cardLimitProgressFillHigh : ""}`}
                  style={{ width: `${usagePercent}%` }}
                  role="progressbar"
                  aria-valuenow={usagePercent}
                  aria-valuemin={0}
                  aria-valuemax={100}
                  aria-label="Limit usage"
                />
              </div>
              <div className={styles.cardLimitProgressLabel}>
                {cardDebtDisplay.toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 })} of{" "}
                {cardLimitDisplay.toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 })} USD
                {cardLimitDisplay > 0 && ` (${usagePercent}%)`}
              </div>
            </div>
          </div>
          <div className={styles.cardBillingNote}>
            <span className={styles.cardBillingLabel}>{t("card_due_date")} (Fkebank):</span>{" "}
            <span className={styles.cardBillingDate}>{dueDateStr}</span>
          </div>
          {firstCard && cardDebtDisplay > 0 && (
            <button type="button" className={styles.submitBtn} onClick={handlePayBill}>
              {t("card_pay_bill")} ({cardDebtDisplay.toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 })} USD)
            </button>
          )}
          <h3 className={styles.cardStatementTitle}>Card statement</h3>
          {wallet.cardTransactions.length === 0 ? (
            <p className={styles.cardStatementEmpty}>No card transactions yet.</p>
          ) : (
            <ul className={styles.cardStatementList}>
              {wallet.cardTransactions.map((tx) => (
                <li key={tx.id} className={styles.cardStatementRow}>
                  <span className={styles.cardStatementDate}>
                    {new Date(tx.created_at_ms).toLocaleString("en-US", { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" })}
                  </span>
                  <span className={styles.cardStatementDesc}>{tx.description || tx.tx_type}</span>
                  <span
                    className={tx.tx_type === "payment" ? styles.cardStatementAmountCredit : styles.cardStatementAmountDebit}
                  >
                    {tx.tx_type === "payment" ? "+" : "-"}
                    {(tx.amount / 100).toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 })} USD
                  </span>
                </li>
              ))}
            </ul>
          )}
        </div>
      )}
    </>
  );
}

function ConvertSection({ wallet }: { wallet: ReturnType<typeof useWallet> }) {
  const { t } = useTranslation("wallet");
  const [fromSymbol, setFromSymbol] = useState("USD");
  const [toSymbol, setToSymbol] = useState("BTC");
  const [fromAmountStr, setFromAmountStr] = useState("");
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<{ type: "success" | "error"; text: string } | null>(null);
  const [, setTick] = useState(0);

  // Refresh rate display periodically
  useEffect(() => {
    const id = setInterval(() => setTick((t) => t + 1), 3000);
    return () => clearInterval(id);
  }, []);

  const rate = getRate(fromSymbol, toSymbol);
  const fromDisplay = parseAmount(fromAmountStr);
  const toDisplay = fromDisplay * rate;

  const handleConfirm = useCallback(async () => {
    setMessage(null);
    if (fromDisplay <= 0) {
      setMessage({ type: "error", text: t("convert_invalid_amount") });
      return;
    }
    if (fromSymbol === toSymbol) {
      setMessage({ type: "error", text: t("convert_same_currency") });
      return;
    }
    const amountCents = Math.round(fromDisplay * 100);
    setBusy(true);
    const err = await wallet.convert(fromSymbol, toSymbol, amountCents);
    setBusy(false);
    if (err === null) {
      setMessage({ type: "success", text: t("convert_success") });
      setFromAmountStr("");
    } else {
      const userMsg = /insufficient/i.test(err)
        ? t("transfer_insufficient_balance")
        : /invalid currency/i.test(err)
          ? t("convert_failed")
          : err;
      setMessage({ type: "error", text: userMsg });
    }
  }, [fromDisplay, fromSymbol, toSymbol, wallet, t]);

  return (
    <>
      <h2 className={styles.mainTitle}>{t("convert_title")}</h2>
      <p className={styles.mainSubtitle}>{t("convert_subtitle")}</p>
      <div className={styles.convertPanel}>
        <div className={styles.convertRow}>
          <input
            type="text"
            className={styles.convertInput}
            inputMode="decimal"
            autoComplete="off"
            value={fromAmountStr}
            onChange={(e) => setFromAmountStr(applyAmountMask(e.target.value, fromSymbol === "USD" ? 2 : 8))}
            placeholder={t("convert_amount_placeholder")}
            disabled={busy}
          />
          <select
            className={styles.convertSelect}
            value={fromSymbol}
            onChange={(e) => setFromSymbol(e.target.value)}
            disabled={busy}
          >
            {CURRENCIES.map((c) => (
              <option key={c.symbol} value={c.symbol}>{c.symbol}</option>
            ))}
          </select>
        </div>
        <div className={styles.convertRate}>
          1 {fromSymbol} = {rate.toFixed(6)} {toSymbol}
        </div>
        <div className={styles.convertRow}>
          <input
            type="text"
            className={styles.convertInput}
            value={fromDisplay > 0 ? toDisplay.toFixed(6) : ""}
            readOnly
            placeholder={t("convert_you_receive")}
          />
          <select
            className={styles.convertSelect}
            value={toSymbol}
            onChange={(e) => setToSymbol(e.target.value)}
            disabled={busy}
          >
            {CURRENCIES.map((c) => (
              <option key={c.symbol} value={c.symbol}>{c.symbol}</option>
            ))}
          </select>
        </div>
        {message && (
          <p className={`${styles.formMessage} ${message.type === "success" ? styles.formMessageSuccess : styles.formMessageError}`}>
            {message.text}
          </p>
        )}
        <button type="button" className={styles.submitBtn} onClick={handleConfirm} disabled={busy}>
          {busy ? t("convert_converting") : t("convert_confirm")}
        </button>
        <div className={styles.convertBalanceSummary}>
          <div className={styles.convertBalanceSummaryTitle}>{t("convert_balance_summary_title")}</div>
          <div className={styles.convertBalanceSummaryRow}>
            {CURRENCIES.map((c) => (
              <span key={c.symbol} className={styles.convertBalanceSummaryItem}>
                <span className={styles.convertBalanceSummarySymbol}>{c.symbol}</span>
                <span className={styles.convertBalanceSummaryAmount}>{wallet.getFormattedBalance(c.symbol)}</span>
              </span>
            ))}
          </div>
        </div>
      </div>
    </>
  );
}

export default function WalletApp() {
  return <WalletContent />;
}
