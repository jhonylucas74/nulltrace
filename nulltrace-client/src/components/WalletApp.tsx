import { useState, useEffect, useMemo } from "react";
import {
  ArrowLeft,
  Check,
  LayoutDashboard,
  Receipt,
  Send,
  Key,
  ArrowLeftRight,
  Image as ImageIcon,
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
import { MOCK_WALLET_BALANCES } from "../lib/walletBalances";
import { MOCK_WALLET_KEYS } from "../lib/walletKeys";
import { getRate } from "../lib/walletConversion";
import { MOCK_WALLET_NFTS, groupNftsByCollection } from "../lib/walletNfts";
import type { WalletTransaction } from "../lib/walletTransactions";
import {
  MOCK_VIRTUAL_CARDS,
  getNextChargeDate,
  nextVirtualCardId,
  formatCardNumber,
  type VirtualCard,
} from "../lib/walletCards";
import { useWallet, parseAmount, applyAmountMask } from "../contexts/WalletContext";
import styles from "./WalletApp.module.css";

type StatementPeriod = "today" | "7d" | "30d" | "all";

function getTransactionTime(tx: WalletTransaction): number {
  if (tx.timestamp != null) return tx.timestamp;
  const parsed = parseStatementDate(tx.date);
  return parsed.getTime();
}

function parseStatementDate(dateStr: string): Date {
  const year = new Date().getFullYear();
  const months: Record<string, number> = {
    Jan: 0, Feb: 1, Mar: 2, Apr: 3, May: 4, Jun: 5,
    Jul: 6, Aug: 7, Sep: 8, Oct: 9, Nov: 10, Dec: 11,
  };
  const parts = dateStr.split(", ");
  const monthDay = (parts[0] ?? "").trim();
  const timePart = (parts[1] ?? "0:00").trim();
  const [monthName, dayStr] = monthDay.split(" ");
  const month = months[monthName ?? ""] ?? 0;
  const day = parseInt(dayStr ?? "1", 10) || 1;
  const [hStr, mStr] = timePart.split(":");
  let hours = parseInt(hStr ?? "0", 10) || 0;
  let minutes = parseInt(mStr ?? "0", 10) || 0;
  const ampm = timePart.toUpperCase();
  if (ampm.includes("PM") && hours < 12) hours += 12;
  if (ampm.includes("AM") && hours === 12) hours = 0;
  return new Date(year, month, day, hours, minutes);
}

function filterStatement(
  transactions: WalletTransaction[],
  period: StatementPeriod,
  searchQuery: string
): WalletTransaction[] {
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
    const time = getTransactionTime(tx);
    if (period !== "all" && time < start) return false;
    if (!q) return true;
    return (
      tx.description.toLowerCase().includes(q) ||
      tx.currency.toLowerCase().includes(q) ||
      tx.amount.toLowerCase().includes(q)
    );
  });
}

const STORAGE_KEY = "wallet-visible-currencies";

function loadVisibleSymbols(): Set<string> {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) {
      const arr = JSON.parse(raw) as string[];
      if (Array.isArray(arr)) return new Set(arr);
    }
  } catch {
    /* ignore */
  }
  return new Set(MOCK_WALLET_BALANCES.map((b) => b.symbol));
}

function saveVisibleSymbols(symbols: Set<string>) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify([...symbols]));
}

type Section = "overview" | "statement" | "transfer" | "keys" | "card" | "convert" | "nfts";
type View = "main" | "select";

function WalletContent() {
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

  const visibleBalances = useMemo(
    () => MOCK_WALLET_BALANCES.filter((b) => visibleSymbols.has(b.symbol)),
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
            {MOCK_WALLET_BALANCES.map((b) => {
              const isChecked = draftSelection.has(b.symbol);
              const isOnlyOne = isChecked && draftSelection.size === 1;
              return (
                <li key={b.symbol}>
                  <button
                    type="button"
                    className={styles.selectItem}
                    onClick={() => !isOnlyOne && toggleDraft(b.symbol)}
                    aria-pressed={isChecked}
                    disabled={isOnlyOne}
                  >
                    <span className={styles.selectItemCheck}>
                      {isChecked ? <Check size={16} /> : null}
                    </span>
                    <span className={styles.selectItemSymbol}>{b.symbol}</span>
                    <span className={styles.selectItemCurrency}>{b.currency}</span>
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
          Overview
        </button>
        <button
          type="button"
          className={`${styles.navItem} ${section === "statement" ? styles.navItemActive : ""}`}
          onClick={() => setSection("statement")}
        >
          <span className={styles.navIcon}><Receipt size={18} /></span>
          Statement
        </button>
        <button
          type="button"
          className={`${styles.navItem} ${section === "transfer" ? styles.navItemActive : ""}`}
          onClick={() => setSection("transfer")}
        >
          <span className={styles.navIcon}><Send size={18} /></span>
          Transfer
        </button>
        <button
          type="button"
          className={`${styles.navItem} ${section === "keys" ? styles.navItemActive : ""}`}
          onClick={() => setSection("keys")}
        >
          <span className={styles.navIcon}><Key size={18} /></span>
          Keys
        </button>
        <button
          type="button"
          className={`${styles.navItem} ${section === "card" ? styles.navItemActive : ""}`}
          onClick={() => setSection("card")}
        >
          <span className={styles.navIcon}><CreditCard size={18} /></span>
          Card
        </button>
        <button
          type="button"
          className={`${styles.navItem} ${section === "convert" ? styles.navItemActive : ""}`}
          onClick={() => setSection("convert")}
        >
          <span className={styles.navIcon}><ArrowLeftRight size={18} /></span>
          Convert
        </button>
        <button
          type="button"
          className={`${styles.navItem} ${section === "nfts" ? styles.navItemActive : ""}`}
          onClick={() => setSection("nfts")}
        >
          <span className={styles.navIcon}><ImageIcon size={18} /></span>
          NFTs
        </button>
      </aside>
      <main className={styles.main}>
        {section === "overview" && (
          <>
            <div className={styles.header}>
              <div>
                <h2 className={styles.mainTitle}>Overview</h2>
                <p className={styles.mainSubtitle}>Your balances.</p>
              </div>
              <button
                type="button"
                className={styles.configBtn}
                onClick={openSelectPage}
                title="Choose which currencies to show"
              >
                <Settings2 size={14} />
                Manage currencies
              </button>
            </div>
            <div className={styles.cards}>
              {visibleBalances.map((item) => (
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
                      USD is trackable and managed by Fkebank.
                    </p>
                  )}
                  {item.symbol !== "USD" && (
                    <p className={styles.cardUsdEquivalent}>
                      ≈{" "}
                      {(
                        (wallet.balances[item.symbol] ?? 0) * getRate(item.symbol, "USD")
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
                <span className={styles.overviewCardDebtLabel}>Card debt (Fkebank)</span>
                <span className={styles.overviewCardDebtValue}>
                  {wallet.cardDebt.toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 })} USD
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
                    {wallet.cardDebt.toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 })} of{" "}
                    {wallet.cardLimit.toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 })} USD
                  </div>
                </div>
              )}
            </div>
          </>
        )}

        {section === "statement" && (
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
                  placeholder="Search description, amount, currency…"
                  value={statementSearch}
                  onChange={(e) => setStatementSearch(e.target.value)}
                  aria-label="Search transactions"
                />
              </div>
            </div>
            <ul className={styles.statementList}>
              {filterStatement(wallet.transactions, statementPeriod, statementSearch).map((tx) => {
                const isCredit = tx.type === "credit" || (tx.type === "convert" && !tx.amount.startsWith("-"));
                const amountClass = isCredit ? styles.statementAmountCredit : styles.statementAmountDebit;
                const displayAmount = tx.amount.startsWith("-") ? tx.amount : `+${tx.amount}`;
                return (
                  <li key={tx.id} className={styles.statementRow}>
                    <span className={styles.statementDate}>{tx.date}</span>
                    <span className={styles.statementDesc}>{tx.description}</span>
                    <span className={`${styles.statementAmount} ${amountClass}`}>
                      {displayAmount} {tx.currency}
                    </span>
                  </li>
                );
              })}
            </ul>
            {filterStatement(wallet.transactions, statementPeriod, statementSearch).length === 0 && (
              <p className={styles.statementEmpty}>No transactions match the filter.</p>
            )}
          </>
        )}

        {section === "transfer" && (
          <TransferSection wallet={wallet} />
        )}

        {section === "keys" && (
          <KeysSection />
        )}

        {section === "card" && (
          <CardSection />
        )}

        {section === "convert" && (
          <ConvertSection wallet={wallet} />
        )}

        {section === "nfts" && (
          <NftsSection />
        )}
      </main>
    </div>
  );
}

function TransferSection({ wallet }: { wallet: ReturnType<typeof useWallet> }) {
  const [recipientKey, setRecipientKey] = useState("");
  const [amountStr, setAmountStr] = useState("");
  const [currency, setCurrency] = useState("USD");
  const [message, setMessage] = useState<{ type: "success" | "error"; text: string } | null>(null);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    setMessage(null);
    const key = recipientKey.trim();
    if (!key) {
      setMessage({ type: "error", text: "Enter recipient key." });
      return;
    }
    const amount = parseAmount(amountStr);
    if (amount <= 0) {
      setMessage({ type: "error", text: "Enter a valid amount." });
      return;
    }
    const ok = wallet.transfer(currency, amount, key);
    if (ok) {
      setMessage({ type: "success", text: "Transfer completed." });
      setRecipientKey("");
      setAmountStr("");
    } else {
      setMessage({ type: "error", text: "Insufficient balance or invalid amount." });
    }
  };

  return (
    <>
      <h2 className={styles.mainTitle}>Transfer</h2>
      <p className={styles.mainSubtitle}>Send to another account using their key.</p>
      <form className={styles.form} onSubmit={handleSubmit}>
        <div className={styles.formGroup}>
          <label className={styles.formLabel} htmlFor="transfer-key">Recipient key</label>
          <input
            id="transfer-key"
            type="text"
            className={styles.formInput}
            value={recipientKey}
            onChange={(e) => setRecipientKey(e.target.value)}
            placeholder="Paste address or key"
          />
        </div>
        <div className={styles.formGroup}>
          <label className={styles.formLabel} htmlFor="transfer-amount">Amount</label>
          <input
            id="transfer-amount"
            type="text"
            inputMode="decimal"
            autoComplete="off"
            className={styles.formInput}
            value={amountStr}
            onChange={(e) => setAmountStr(applyAmountMask(e.target.value, currency === "USD" ? 2 : 8))}
            placeholder="0,00 or 0.00"
          />
        </div>
        <div className={styles.formGroup}>
          <label className={styles.formLabel} htmlFor="transfer-currency">Currency</label>
          <select
            id="transfer-currency"
            className={styles.formSelect}
            value={currency}
            onChange={(e) => setCurrency(e.target.value)}
          >
            {MOCK_WALLET_BALANCES.map((b) => (
              <option key={b.symbol} value={b.symbol}>{b.symbol}</option>
            ))}
          </select>
        </div>
        {message && (
          <p className={`${styles.formMessage} ${message.type === "success" ? styles.formMessageSuccess : styles.formMessageError}`}>
            {message.text}
          </p>
        )}
        <button type="submit" className={styles.submitBtn}>Send</button>
      </form>
    </>
  );
}

function KeysSection() {
  const [copied, setCopied] = useState<"usd" | "crypto" | null>(null);

  const copyToClipboard = async (text: string, which: "usd" | "crypto") => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(which);
      setTimeout(() => setCopied(null), 2000);
    } catch {
      /* ignore */
    }
  };

  return (
    <>
      <h2 className={styles.mainTitle}>Keys</h2>
      <p className={styles.mainSubtitle}>Your receive keys and addresses.</p>

      <div className={styles.keyBlock}>
        <div className={styles.keyBlockTitle}>USD receive key (Fkebank)</div>
        <div className={styles.keyValue}>{MOCK_WALLET_KEYS.usdReceiveKey}</div>
        <p className={styles.keyExplanation}>
          Your USD balance is managed by Fkebank. Use this key to receive USD. Transfers are trackable and can be used for instant payments.
        </p>
        <div className={styles.keyCopyWrap}>
          <button
            type="button"
            className={styles.keyCopyBtn}
            onClick={() => copyToClipboard(MOCK_WALLET_KEYS.usdReceiveKey, "usd")}
            aria-label="Copy USD key"
          >
            <Copy size={14} />
            {copied === "usd" ? "Copied" : "Copy"}
          </button>
        </div>
      </div>

      <div className={styles.keyBlock}>
        <div className={styles.keyBlockTitle}>Crypto wallet address</div>
        <div className={styles.keyValue}>{MOCK_WALLET_KEYS.cryptoAddress}</div>
        <p className={styles.keyExplanation}>
          This is your public wallet address for crypto assets. Anyone can send tokens to this address; only you can spend them. Do not share private keys.
        </p>
        <div className={styles.keyCopyWrap}>
          <button
            type="button"
            className={styles.keyCopyBtn}
            onClick={() => copyToClipboard(MOCK_WALLET_KEYS.cryptoAddress, "crypto")}
            aria-label="Copy crypto address"
          >
            <Copy size={14} />
            {copied === "crypto" ? "Copied" : "Copy"}
          </button>
        </div>
      </div>
    </>
  );
}

type CardTab = "cards" | "statement";

function CardSection() {
  const wallet = useWallet();
  const [cardTab, setCardTab] = useState<CardTab>("statement");
  const [cards, setCards] = useState<VirtualCard[]>(MOCK_VIRTUAL_CARDS);
  const [cvvRevealed, setCvvRevealed] = useState<string | null>(null);
  const [copiedCardId, setCopiedCardId] = useState<string | null>(null);

  const nextCharge = getNextChargeDate();
  const usageRatio = wallet.cardLimit > 0 ? Math.min(1, wallet.cardDebt / wallet.cardLimit) : 0;
  const usagePercent = Math.round(usageRatio * 100);
  const nextChargeStr = nextCharge.toLocaleDateString("en-US", { weekday: "short", month: "short", day: "numeric", year: "numeric" });

  const copyCardNumber = async (card: VirtualCard) => {
    try {
      await navigator.clipboard.writeText(card.number.replace(/\s/g, ""));
      setCopiedCardId(card.id);
      setTimeout(() => setCopiedCardId(null), 2000);
    } catch {
      /* ignore */
    }
  };

  const toggleCvv = (cardId: string) => {
    const next = cvvRevealed === cardId ? null : cardId;
    setCvvRevealed(next);
    if (next === cardId) {
      setTimeout(() => setCvvRevealed((c) => (c === cardId ? null : c)), 5000);
    }
  };

  const handleCreateCard = () => {
    setCards((prev) => [
      ...prev,
      {
        id: nextVirtualCardId(),
        last4: String(Math.floor(1000 + Math.random() * 9000)),
        number: "411111111111" + String(Math.floor(1000 + Math.random() * 9000)),
        expiryMonth: 3,
        expiryYear: 2029,
        holderName: "Nulltrace User",
        cvv: String(Math.floor(100 + Math.random() * 900)),
        label: "Virtual " + (prev.length + 1),
      },
    ]);
  };

  const handleDeleteCard = (cardId: string) => {
    setCards((prev) => prev.filter((c) => c.id !== cardId));
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
          <div className={styles.cardSummaryBlock}>
            <div className={styles.cardSummaryRow}>
              <span className={styles.cardSummaryLabel}>Current debt</span>
              <span className={styles.cardSummaryValue}>
                {wallet.cardDebt.toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 })} USD
              </span>
            </div>
            <div className={styles.cardSummaryRow}>
              <span className={styles.cardSummaryLabel}>Credit limit</span>
              <span className={styles.cardSummaryValue}>
                {wallet.cardLimit.toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 })} USD
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
                {wallet.cardDebt.toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 })} of{" "}
                {wallet.cardLimit.toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 })} USD
                {wallet.cardLimit > 0 && ` (${usagePercent}%)`}
              </div>
            </div>
          </div>
          <div className={styles.cardBillingNote}>
            <span className={styles.cardBillingLabel}>Next invoice charge (Fkebank):</span>{" "}
            <span className={styles.cardBillingDate}>{nextChargeStr}</span>
          </div>
          <div className={styles.virtualCardList}>
            {cards.map((card) => (
              <div key={card.id} className={styles.virtualCard}>
                <div className={styles.virtualCardHeader}>
                  <CreditCard size={18} className={styles.virtualCardIcon} aria-hidden="true" />
                  <span className={styles.virtualCardLabel}>{card.label ?? "Card"}</span>
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
                  <span className={styles.virtualCardNumber}>{formatCardNumber(card.number)}</span>
                  <button
                    type="button"
                    className={styles.virtualCardCopyBtn}
                    onClick={() => copyCardNumber(card)}
                    aria-label="Copy card number"
                    title="Copy number"
                  >
                    <Copy size={14} />
                    {copiedCardId === card.id ? "Copied" : "Copy"}
                  </button>
                </div>
                <div className={styles.virtualCardMeta}>
                  <span>{card.holderName}</span>
                  <span className={styles.virtualCardValidity}>
                    Valid thru {String(card.expiryMonth).padStart(2, "0")}/{card.expiryYear}
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
                    title={cvvRevealed === card.id ? "Hide" : "Show"}
                  >
                    {cvvRevealed === card.id ? <EyeOff size={14} /> : <Eye size={14} />}
                  </button>
                </div>
              </div>
            ))}
          </div>
          <button type="button" className={styles.addCardBtn} onClick={handleCreateCard}>
            <Plus size={18} />
            Create virtual card
          </button>
        </>
      )}
      {cardTab === "statement" && (
        <div className={styles.cardStatementSection}>
          <h3 className={styles.cardStatementTitle}>Card statement</h3>
          {wallet.cardTransactions.length === 0 ? (
            <p className={styles.cardStatementEmpty}>No card transactions yet.</p>
          ) : (
            <ul className={styles.cardStatementList}>
              {wallet.cardTransactions.map((tx) => (
                <li key={tx.id} className={styles.cardStatementRow}>
                  <span className={styles.cardStatementDate}>{tx.date}</span>
                  <span className={styles.cardStatementDesc}>{tx.description}</span>
                  <span
                    className={
                      tx.type === "payment"
                        ? styles.cardStatementAmountCredit
                        : styles.cardStatementAmountDebit
                    }
                  >
                    {tx.type === "payment" ? "+" : "-"}
                    {tx.amount} USD
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
  const [fromSymbol, setFromSymbol] = useState("USD");
  const [toSymbol, setToSymbol] = useState("BTC");
  const [fromAmountStr, setFromAmountStr] = useState("");
  const [message, setMessage] = useState<{ type: "success" | "error"; text: string } | null>(null);
  const [, setTick] = useState(0);

  // Refresh rate display periodically (variable fake rate)
  useEffect(() => {
    const id = setInterval(() => setTick((t) => t + 1), 3000);
    return () => clearInterval(id);
  }, []);

  const rate = getRate(fromSymbol, toSymbol);
  const fromAmount = parseAmount(fromAmountStr);
  const toAmount = fromAmount * rate;

  const handleConfirm = () => {
    setMessage(null);
    if (fromAmount <= 0) {
      setMessage({ type: "error", text: "Enter a valid amount." });
      return;
    }
    const ok = wallet.convert(fromSymbol, toSymbol, fromAmount, rate);
    if (ok) {
      setMessage({ type: "success", text: "Conversion completed." });
      setFromAmountStr("");
    } else {
      setMessage({ type: "error", text: "Insufficient balance or invalid amount." });
    }
  };

  return (
    <>
      <h2 className={styles.mainTitle}>Convert</h2>
      <p className={styles.mainSubtitle}>Exchange between your currencies at the current rate.</p>
      <div className={styles.convertPanel}>
        <div className={styles.convertRow}>
          <input
            type="text"
            className={styles.convertInput}
            inputMode="decimal"
            autoComplete="off"
            value={fromAmountStr}
            onChange={(e) => setFromAmountStr(applyAmountMask(e.target.value, fromSymbol === "USD" ? 2 : 8))}
            placeholder="Amount (e.g. 10,50)"
          />
          <select
            className={styles.convertSelect}
            value={fromSymbol}
            onChange={(e) => setFromSymbol(e.target.value)}
          >
            {MOCK_WALLET_BALANCES.map((b) => (
              <option key={b.symbol} value={b.symbol}>{b.symbol}</option>
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
            value={fromAmount > 0 ? toAmount.toFixed(6) : ""}
            readOnly
            placeholder="You receive"
          />
          <select
            className={styles.convertSelect}
            value={toSymbol}
            onChange={(e) => setToSymbol(e.target.value)}
          >
            {MOCK_WALLET_BALANCES.map((b) => (
              <option key={b.symbol} value={b.symbol}>{b.symbol}</option>
            ))}
          </select>
        </div>
        {message && (
          <p className={`${styles.formMessage} ${message.type === "success" ? styles.formMessageSuccess : styles.formMessageError}`}>
            {message.text}
          </p>
        )}
        <button type="button" className={styles.submitBtn} onClick={handleConfirm}>
          Confirm conversion
        </button>
        <div className={styles.convertBalanceSummary}>
          <div className={styles.convertBalanceSummaryTitle}>Your balances</div>
          <div className={styles.convertBalanceSummaryRow}>
            {MOCK_WALLET_BALANCES.map((b) => (
              <span key={b.symbol} className={styles.convertBalanceSummaryItem}>
                <span className={styles.convertBalanceSummarySymbol}>{b.symbol}</span>
                <span className={styles.convertBalanceSummaryAmount}>{wallet.getFormattedBalance(b.symbol)}</span>
              </span>
            ))}
          </div>
        </div>
      </div>
    </>
  );
}

function NftsSection() {
  const byCollection = useMemo(() => groupNftsByCollection(MOCK_WALLET_NFTS), []);

  if (MOCK_WALLET_NFTS.length === 0) {
    return (
      <>
        <h2 className={styles.mainTitle}>NFTs</h2>
        <p className={styles.mainSubtitle}>Your digital collectibles.</p>
        <div className={styles.nftEmpty}>No NFTs yet.</div>
      </>
    );
  }

  return (
    <>
      <h2 className={styles.mainTitle}>NFTs</h2>
      <p className={styles.mainSubtitle}>Your digital collectibles by collection.</p>
      {Array.from(byCollection.entries()).map(([collectionName, nfts]) => (
        <div key={collectionName} className={styles.nftSection}>
          <div className={styles.nftSectionTitle}>{collectionName}</div>
          <div className={styles.nftGrid}>
            {nfts.map((nft) => (
              <div key={nft.id} className={styles.nftCard}>
                <div className={styles.nftImageWrap}>
                  <img
                    src={nft.imageUrl}
                    alt=""
                    className={styles.nftImage}
                  />
                </div>
                <div className={styles.nftInfo}>
                  <div className={styles.nftName}>{nft.name}</div>
                  <div className={styles.nftCollection}>{nft.collection}</div>
                </div>
              </div>
            ))}
          </div>
        </div>
      ))}
    </>
  );
}

export default function WalletApp() {
  return <WalletContent />;
}
