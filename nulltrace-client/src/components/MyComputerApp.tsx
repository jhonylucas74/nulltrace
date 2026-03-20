import { useState, useCallback, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import {
  LayoutDashboard,
  Cpu,
  MemoryStick,
  HardDrive,
  Wifi,
  Monitor,
  Loader2,
} from "lucide-react";
import { useAuth } from "../contexts/AuthContext";
import { useGrpc } from "../contexts/GrpcContext";
import { useNullCloud } from "../contexts/NullCloudContext";
import { usePaymentFeedback } from "../contexts/PaymentFeedbackContext";
import { useWallet } from "../contexts/WalletContext";
import Modal from "./Modal";
import styles from "./MyComputerApp.module.css";

type NavSection = "config" | "cpu" | "ram" | "storage" | "internet";

/** Server-provided machine config (Current configuration section only). */
interface ServerMachineConfig {
  cpuCores: number;
  ramGib: number;
  diskTotalGib: number;
  internetPlanId: string;
  internetPlanNextBilling: number | null;
}

function formatBillingDate(ts: number): string {
  return new Date(ts).toLocaleDateString("en-US", {
    month: "short",
    day: "numeric",
    year: "numeric",
  });
}

export default function MyComputerApp() {
  const { t } = useTranslation("my_computer");
  const { t: tCommon } = useTranslation("common");
  const { token } = useAuth();
  const cloud = useNullCloud();
  const grpc = useGrpc();
  const wallet = useWallet();
  const { triggerFlyToWallet } = usePaymentFeedback();
  const [section, setSection] = useState<NavSection>("config");
  const machine = cloud.localMachine;

  const [serverConfig, setServerConfig] = useState<ServerMachineConfig | null>(null);
  const [configLoading, setConfigLoading] = useState(true);
  const [configError, setConfigError] = useState<string | null>(null);
  const [insufficientBalanceModalOpen, setInsufficientBalanceModalOpen] = useState(false);
  const [upgradeErrorModalOpen, setUpgradeErrorModalOpen] = useState(false);
  const [upgradeErrorMessage, setUpgradeErrorMessage] = useState("");
  const [upgradingKey, setUpgradingKey] = useState<string | null>(null);

  const fetchServerConfig = useCallback(async () => {
    if (!token) {
      setServerConfig(null);
      setConfigLoading(false);
      setConfigError(null);
      return;
    }
    setConfigLoading(true);
    setConfigError(null);
    try {
      const res = await invoke<{
        cpu_cores: number;
        memory_mb: number;
        disk_mb: number;
        internet_plan_id?: string;
        internet_plan_next_billing_ms?: number;
        error_message: string;
      }>("grpc_sysinfo", { token });
      if (res.error_message) {
        setConfigError(res.error_message);
        setServerConfig(null);
      } else {
        setServerConfig({
          cpuCores: res.cpu_cores,
          ramGib: Math.round((res.memory_mb / 1024) * 10) / 10,
          diskTotalGib: Math.round((res.disk_mb / 1024) * 10) / 10,
          internetPlanId: res.internet_plan_id ?? "basic",
          internetPlanNextBilling:
            res.internet_plan_next_billing_ms != null && res.internet_plan_next_billing_ms > 0
              ? res.internet_plan_next_billing_ms
              : null,
        });
        setConfigError(null);
      }
    } catch (e) {
      setConfigError(e instanceof Error ? e.message : String(e));
      setServerConfig(null);
    } finally {
      setConfigLoading(false);
    }
  }, [token]);

  useEffect(() => {
    fetchServerConfig();
  }, [fetchServerConfig]);

  const currentCpu = serverConfig?.cpuCores ?? machine.cpuCores;
  const currentRam = serverConfig?.ramGib ?? machine.ramGib;
  const currentDisk = serverConfig?.diskTotalGib ?? machine.diskTotalGib;

  async function handleUpgradeClick(
    event: React.MouseEvent,
    upgradeType: "cpu" | "ram" | "disk",
    newValue: number,
    priceUsd: number
  ) {
    if (!token) return;
    const priceCents = priceUsd * 100;
    const usdBalanceCents = wallet.balances["USD"] ?? 0;
    if (usdBalanceCents < priceCents) {
      setInsufficientBalanceModalOpen(true);
      return;
    }
    const key = `${upgradeType}-${newValue}`;
    setUpgradingKey(key);
    try {
      const res = await grpc.upgradeVm(token, upgradeType, newValue);
      if (res.success) {
        await fetchServerConfig();
        triggerFlyToWallet(event.clientX, event.clientY);
        wallet.refreshBalances();
      } else {
        setUpgradeErrorMessage(res.error_message || t("upgrade_error"));
        setUpgradeErrorModalOpen(true);
      }
    } catch (e) {
      setUpgradeErrorMessage(e instanceof Error ? e.message : String(e));
      setUpgradeErrorModalOpen(true);
    } finally {
      setUpgradingKey(null);
    }
  }

  return (
    <div className={styles.appWithSidebar}>
      <aside className={styles.sidebar}>
        <div className={styles.sidebarSection}>{t("app_name")}</div>
        <button
          type="button"
          className={`${styles.navItem} ${section === "config" ? styles.navItemActive : ""}`}
          onClick={() => setSection("config")}
        >
          <span className={styles.navIcon}>
            <LayoutDashboard size={18} />
          </span>
          {t("nav_current_config")}
        </button>
        <button
          type="button"
          className={`${styles.navItem} ${section === "cpu" ? styles.navItemActive : ""}`}
          onClick={() => setSection("cpu")}
        >
          <span className={styles.navIcon}>
            <Cpu size={18} />
          </span>
          {t("nav_cpu")}
        </button>
        <button
          type="button"
          className={`${styles.navItem} ${section === "ram" ? styles.navItemActive : ""}`}
          onClick={() => setSection("ram")}
        >
          <span className={styles.navIcon}>
            <MemoryStick size={18} />
          </span>
          {t("nav_ram")}
        </button>
        <button
          type="button"
          className={`${styles.navItem} ${section === "storage" ? styles.navItemActive : ""}`}
          onClick={() => setSection("storage")}
        >
          <span className={styles.navIcon}>
            <HardDrive size={18} />
          </span>
          {t("nav_storage")}
        </button>
        <button
          type="button"
          className={`${styles.navItem} ${section === "internet" ? styles.navItemActive : ""}`}
          onClick={() => setSection("internet")}
        >
          <span className={styles.navIcon}>
            <Wifi size={18} />
          </span>
          {t("nav_internet")}
        </button>
      </aside>
      <main className={styles.main}>
        {section === "config" && (
          <>
            <h2 className={styles.mainTitle}>{t("title_current_config")}</h2>
            <p className={styles.mainSubtitle}>{t("subtitle_current_config")}</p>
            {configLoading && (
              <div className={styles.configLoading}>
                <p className={styles.configLoadingText}>{t("config_loading")}</p>
                <Loader2 size={28} className={styles.configLoadingSpinner} aria-hidden />
              </div>
            )}
            {!configLoading && !token && (
              <p className={styles.configError}>{t("config_login_required")}</p>
            )}
            {!configLoading && token && configError && (
              <p className={styles.configError}>{t("config_error")}</p>
            )}
            {!configLoading && serverConfig != null && (
              <div className={styles.card}>
                <div className={styles.cardTitleRow}>
                  <Monitor size={20} className={styles.cardTitleIcon} />
                  <span className={styles.cardTitle}>{t("card_your_machine")}</span>
                </div>
                <p className={styles.cardDesc}>{t("card_desc")}</p>
                <div className={styles.overviewSpecGrid}>
                  <button
                    type="button"
                    className={`${styles.overviewSpecItem} ${styles.overviewSpecItemClickable}`}
                    onClick={() => setSection("cpu")}
                  >
                    <div className={styles.overviewSpecIconWrap}>
                      <Cpu size={22} className={styles.overviewSpecIconCpu} />
                    </div>
                    <div className={styles.overviewSpecLabel}>{t("label_cpu")}</div>
                    <div className={styles.overviewSpecValue}>
                      {t("value_cores", { count: serverConfig.cpuCores })}
                    </div>
                  </button>
                  <button
                    type="button"
                    className={`${styles.overviewSpecItem} ${styles.overviewSpecItemClickable}`}
                    onClick={() => setSection("ram")}
                  >
                    <div className={styles.overviewSpecIconWrap}>
                      <MemoryStick size={22} className={styles.overviewSpecIconRam} />
                    </div>
                    <div className={styles.overviewSpecLabel}>{t("label_memory")}</div>
                    <div className={styles.overviewSpecValue}>
                      {t("value_gib_ram", { count: serverConfig.ramGib })}
                    </div>
                  </button>
                  <button
                    type="button"
                    className={`${styles.overviewSpecItem} ${styles.overviewSpecItemClickable}`}
                    onClick={() => setSection("storage")}
                  >
                    <div className={styles.overviewSpecIconWrap}>
                      <HardDrive size={22} className={styles.overviewSpecIconDisk} />
                    </div>
                    <div className={styles.overviewSpecLabel}>{t("label_storage")}</div>
                    <div className={styles.overviewSpecValue}>
                      {t("value_gib_storage", { count: serverConfig.diskTotalGib })}
                    </div>
                  </button>
                  <button
                    type="button"
                    className={`${styles.overviewSpecItem} ${styles.overviewSpecItemClickable}`}
                    onClick={() => setSection("internet")}
                  >
                    <div className={styles.overviewSpecIconWrap}>
                      <Wifi size={22} className={styles.overviewSpecIconInternet} />
                    </div>
                    <div className={styles.overviewSpecLabel}>{t("label_internet")}</div>
                    <div className={styles.overviewSpecValue}>
                      {(() => {
                        const plan = cloud.getInternetPlanById(
                          serverConfig.internetPlanId || "basic"
                        );
                        return plan
                          ? `${plan.name} · ${plan.speedMbps} Mbps`
                          : "—";
                      })()}
                    </div>
                    {serverConfig.internetPlanNextBilling != null && (
                      <div className={styles.overviewSpecMeta}>
                        {t("next_billing", {
                          date: formatBillingDate(serverConfig.internetPlanNextBilling),
                        })}
                      </div>
                    )}
                  </button>
                </div>
              </div>
            )}
          </>
        )}

        {section === "cpu" && (
          <>
            <h2 className={styles.mainTitle}>{t("nav_cpu")}</h2>
            <p className={styles.mainSubtitle}>{t("category_processor_desc")}</p>
            <div className={`${styles.card} ${styles.categoryCpu}`}>
              <div className={styles.categoryHeader}>
                <div className={styles.categoryIconWrap}>
                  <Cpu size={20} className={styles.categoryIconCpu} />
                </div>
                <div>
                  <h3 className={styles.categoryTitle}>{t("category_processor")}</h3>
                  <p className={styles.categoryDesc}>{t("category_processor_desc")}</p>
                </div>
              </div>
              <div className={styles.currentSpec}>
                {t("current_spec")}: {currentCpu} vCPU
              </div>
              <div className={styles.productGrid}>
                {cloud.cpuUpgrades.map((o) => {
                  const isInferior = o.value <= currentCpu;
                  const upgrading = upgradingKey === `cpu-${o.value}`;
                  return (
                    <div
                      key={o.value}
                      className={`${styles.productCard} ${isInferior ? styles.productCardInferior : ""}`.trim()}
                    >
                      <div className={styles.productCardHeader}>
                        <span className={styles.productCardTitle}>{o.label}</span>
                      </div>
                      <div className={styles.productCardSpecs}>
                        {isInferior
                          ? `${o.value} vCPU`
                          : `${currentCpu} → ${o.value} vCPU`}
                      </div>
                      <div className={styles.productCardPrice}>
                        {o.priceUsd === 0 ? t("free") : `$${o.priceUsd}`}
                      </div>
                      <div className={styles.productCardMeta}>{t("one_time")}</div>
                      {isInferior ? (
                        <span className={styles.productCardPriceMuted}>
                          {o.value === currentCpu
                            ? t("current")
                            : t("lower_tier")}
                        </span>
                      ) : (
                        <button
                          type="button"
                          className={styles.btnUpgrade}
                          disabled={!!upgrading}
                          onClick={(e) => handleUpgradeClick(e, "cpu", o.value, o.priceUsd)}
                        >
                          {upgrading ? (
                            <Loader2 size={16} className={styles.btnUpgradeSpinner} aria-hidden />
                          ) : (
                            t("add_upgrade")
                          )}
                        </button>
                      )}
                    </div>
                  );
                })}
              </div>
            </div>
          </>
        )}

        {section === "ram" && (
          <>
            <h2 className={styles.mainTitle}>{t("nav_ram")}</h2>
            <p className={styles.mainSubtitle}>{t("category_memory_desc")}</p>
            <div className={`${styles.card} ${styles.categoryRam}`}>
              <div className={styles.categoryHeader}>
                <div className={styles.categoryIconWrap}>
                  <MemoryStick size={20} className={styles.categoryIconRam} />
                </div>
                <div>
                  <h3 className={styles.categoryTitle}>{t("category_memory")}</h3>
                  <p className={styles.categoryDesc}>{t("category_memory_desc")}</p>
                </div>
              </div>
              <div className={styles.currentSpec}>
                {t("current_spec")}: {currentRam} GiB RAM
              </div>
              <div className={styles.productGrid}>
                {cloud.ramUpgrades.map((o) => {
                  const isInferior = o.value <= currentRam;
                  const upgrading = upgradingKey === `ram-${o.value}`;
                  return (
                    <div
                      key={o.value}
                      className={`${styles.productCard} ${isInferior ? styles.productCardInferior : ""}`.trim()}
                    >
                      <div className={styles.productCardHeader}>
                        <span className={styles.productCardTitle}>{o.label}</span>
                      </div>
                      <div className={styles.productCardSpecs}>
                        {isInferior
                          ? `${o.value} GiB RAM`
                          : `${currentRam} → ${o.value} GiB RAM`}
                      </div>
                      <div className={styles.productCardPrice}>
                        {o.priceUsd === 0 ? t("free") : `$${o.priceUsd}`}
                      </div>
                      <div className={styles.productCardMeta}>{t("one_time")}</div>
                      {isInferior ? (
                        <span className={styles.productCardPriceMuted}>
                          {o.value === currentRam
                            ? t("current")
                            : t("lower_tier")}
                        </span>
                      ) : (
                        <button
                          type="button"
                          className={styles.btnUpgrade}
                          disabled={!!upgrading}
                          onClick={(e) => handleUpgradeClick(e, "ram", o.value, o.priceUsd)}
                        >
                          {upgrading ? (
                            <Loader2 size={16} className={styles.btnUpgradeSpinner} aria-hidden />
                          ) : (
                            t("add_upgrade")
                          )}
                        </button>
                      )}
                    </div>
                  );
                })}
              </div>
            </div>
          </>
        )}

        {section === "storage" && (
          <>
            <h2 className={styles.mainTitle}>{t("nav_storage")}</h2>
            <p className={styles.mainSubtitle}>{t("category_storage_desc")}</p>
            <div className={`${styles.card} ${styles.categoryDisk}`}>
              <div className={styles.categoryHeader}>
                <div className={styles.categoryIconWrap}>
                  <HardDrive size={20} className={styles.categoryIconDisk} />
                </div>
                <div>
                  <h3 className={styles.categoryTitle}>{t("category_storage")}</h3>
                  <p className={styles.categoryDesc}>{t("category_storage_desc")}</p>
                </div>
              </div>
              <div className={styles.currentSpec}>
                {t("current_spec")}: {currentDisk} GiB
              </div>
              <div className={styles.productGrid}>
                {cloud.diskUpgrades.map((o) => {
                  const isInferior = o.value <= currentDisk;
                  const upgrading = upgradingKey === `disk-${o.value}`;
                  return (
                    <div
                      key={o.value}
                      className={`${styles.productCard} ${isInferior ? styles.productCardInferior : ""}`.trim()}
                    >
                      <div className={styles.productCardHeader}>
                        <span className={styles.productCardTitle}>{o.label}</span>
                      </div>
                      <div className={styles.productCardSpecs}>
                        {isInferior
                          ? `${o.value} GiB`
                          : `${currentDisk} → ${o.value} GiB`}
                      </div>
                      <div className={styles.productCardPrice}>
                        {o.priceUsd === 0 ? t("free") : `$${o.priceUsd}`}
                      </div>
                      <div className={styles.productCardMeta}>{t("one_time")}</div>
                      {isInferior ? (
                        <span className={styles.productCardPriceMuted}>
                          {o.value === currentDisk
                            ? t("current")
                            : t("lower_tier")}
                        </span>
                      ) : (
                        <button
                          type="button"
                          className={styles.btnUpgrade}
                          disabled={!!upgrading}
                          onClick={(e) => handleUpgradeClick(e, "disk", o.value, o.priceUsd)}
                        >
                          {upgrading ? (
                            <Loader2 size={16} className={styles.btnUpgradeSpinner} aria-hidden />
                          ) : (
                            t("add_upgrade")
                          )}
                        </button>
                      )}
                    </div>
                  );
                })}
              </div>
            </div>
          </>
        )}

        {section === "internet" && (
          <>
            <h2 className={styles.mainTitle}>{t("nav_internet")}</h2>
            <p className={styles.mainSubtitle}>{t("category_internet_desc")}</p>
            <div className={`${styles.card} ${styles.categoryInternet}`}>
              <div className={styles.categoryHeader}>
                <div className={styles.categoryIconWrap}>
                  <Wifi size={20} className={styles.categoryIconInternet} />
                </div>
                <div>
                  <h3 className={styles.categoryTitle}>{t("category_internet")}</h3>
                  <p className={styles.categoryDesc}>{t("category_internet_desc")}</p>
                </div>
              </div>
              <div className={styles.currentSpec}>
                {t("current_spec")}:{" "}
                {cloud.getInternetPlanById(machine.internetPlanId ?? "basic")?.name ??
                  "—"}{" "}
                (
                {cloud.getInternetPlanById(machine.internetPlanId ?? "basic")
                  ?.speedMbps ?? 0}{" "}
                Mbps)
              </div>
              <div className={styles.productGrid}>
                {cloud.internetPlans.map((plan) => {
                  const isCurrent =
                    (machine.internetPlanId ?? "basic") === plan.id;
                  return (
                    <div key={plan.id} className={styles.productCard}>
                      {plan.badge && (
                        <span className={styles.productCardBadge}>
                          {plan.badge}
                        </span>
                      )}
                      <div className={styles.productCardHeader}>
                        <span className={styles.productCardTitle}>
                          {plan.name}
                        </span>
                      </div>
                      <div className={styles.productCardSpecs}>
                        {plan.speedMbps} Mbps
                      </div>
                      <div className={styles.productCardPrice}>
                        {plan.weeklyPriceUsd === 0
                          ? t("free")
                          : `$${plan.weeklyPriceUsd}`}
                        <span className={styles.productCardPriceUnit}>/wk</span>
                      </div>
                      <div className={styles.productCardMeta}>
                        {t("weekly_billing")}
                      </div>
                      {isCurrent ? (
                        <span className={styles.productCardCurrent}>
                          {t("current_plan")}
                        </span>
                      ) : (
                        <button
                          type="button"
                          className={styles.btnUpgradeDisabled}
                          disabled
                        >
                          {t("upgrade_coming_soon")}
                        </button>
                      )}
                    </div>
                  );
                })}
              </div>
            </div>
          </>
        )}
      </main>

      <Modal
        open={insufficientBalanceModalOpen}
        onClose={() => setInsufficientBalanceModalOpen(false)}
        title={t("insufficient_usd_balance")}
        primaryButton={{ label: tCommon("ok"), onClick: () => setInsufficientBalanceModalOpen(false) }}
      >
        <p className={styles.modalMessage}>{t("insufficient_usd_balance")}</p>
      </Modal>
      <Modal
        open={upgradeErrorModalOpen}
        onClose={() => setUpgradeErrorModalOpen(false)}
        title={t("upgrade_error")}
        primaryButton={{ label: tCommon("ok"), onClick: () => setUpgradeErrorModalOpen(false) }}
      >
        <p className={styles.modalMessage}>{upgradeErrorMessage}</p>
      </Modal>
    </div>
  );
}
