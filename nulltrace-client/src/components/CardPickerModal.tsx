import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { CreditCard, Loader2 } from "lucide-react";
import Modal from "./Modal";
import { useGrpc } from "../contexts/GrpcContext";
import type { GrpcWalletCard } from "../contexts/GrpcContext";
import styles from "./CardPickerModal.module.css";

/** Sanitized card display: **** **** **** 2020 */
function formatSanitizedCard(last4: string): string {
  return `**** **** **** ${last4}`;
}

export interface CardPickerModalProps {
  open: boolean;
  onClose: () => void;
  onConfirm: (card: GrpcWalletCard) => void;
  origin: string;
  requestId: string;
  token: string;
}

export default function CardPickerModal({
  open,
  onClose,
  onConfirm,
  origin,
  token,
}: CardPickerModalProps) {
  const { t } = useTranslation("browser");
  const grpc = useGrpc();
  const [cards, setCards] = useState<GrpcWalletCard[]>([]);
  const [loading, setLoading] = useState(false);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open || !token) return;
    setLoading(true);
    setError(null);
    setSelectedId(null);
    grpc
      .getWalletCards(token)
      .then((res) => {
        setCards(res.cards ?? []);
        if ((res.cards ?? []).length > 0) {
          setSelectedId(res.cards![0].id);
        }
      })
      .catch(() => setError("Failed to load cards"))
      .finally(() => setLoading(false));
  }, [open, token, grpc]);

  const handleConfirm = () => {
    if (!selectedId) return;
    const card = cards.find((c) => c.id === selectedId);
    if (card) {
      onConfirm(card);
    }
  };

  return (
    <Modal
      open={open}
      onClose={onClose}
      title={t("card_picker_title")}
      primaryButton={{
        label: t("card_picker_confirm"),
        onClick: handleConfirm,
        disabled: loading || cards.length === 0 || !selectedId,
      }}
      secondaryButton={{ label: t("card_picker_cancel"), onClick: onClose }}
    >
      <div className={styles.content}>
        <p className={styles.description}>{t("card_picker_description", { origin })}</p>
        {loading && (
          <div className={styles.loading}>
            <Loader2 size={24} className={styles.spinner} aria-hidden />
            <span>Loading…</span>
          </div>
        )}
        {error && <p className={styles.error} role="alert">{error}</p>}
        {!loading && !error && cards.length === 0 && (
          <p className={styles.noCards}>{t("card_picker_no_cards")}</p>
        )}
        {!loading && !error && cards.length > 0 && (
          <ul className={styles.cardList} role="listbox" aria-label="Select a card">
            {cards.map((card) => {
              const last4 = card.last4 ?? "";
              const label = card.label || "Card";
              const isSelected = selectedId === card.id;
              return (
                <li key={card.id}>
                  <button
                    type="button"
                    className={`${styles.cardItem} ${isSelected ? styles.cardItemSelected : ""}`}
                    onClick={() => setSelectedId(card.id)}
                    role="option"
                    aria-selected={isSelected}
                  >
                    <CreditCard size={20} className={styles.cardIcon} aria-hidden />
                    <span className={styles.cardLabel}>{label}</span>
                    <span className={styles.cardNumber}>{formatSanitizedCard(last4)}</span>
                  </button>
                </li>
              );
            })}
          </ul>
        )}
      </div>
    </Modal>
  );
}
