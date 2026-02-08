import { useState } from "react";
import { LayoutDashboard, ShoppingCart, Server, Monitor, Cpu, MemoryStick, HardDrive, Wifi, Globe, Calendar, KeyRound } from "lucide-react";
import { useNullCloud } from "../contexts/NullCloudContext";
import { useWallet } from "../contexts/WalletContext";
import { getPlanById } from "../lib/nullcloudData";
import styles from "./NullCloudApp.module.css";

type Section = "overview" | "shop" | "vps";
type ShopTab = "machine" | "vps";

function formatBillingDate(ts: number): string {
  return new Date(ts).toLocaleDateString("en-US", { month: "short", day: "numeric", year: "numeric" });
}

export default function NullCloudApp() {
  const cloud = useNullCloud();
  const wallet = useWallet();
  const [section, setSection] = useState<Section>("overview");
  const [shopTab, setShopTab] = useState<ShopTab>("machine");
  const [newVpsId, setNewVpsId] = useState<string | null>(null);
  const [sshUser, setSshUser] = useState("");
  const [sshPassword, setSshPassword] = useState("");
  const [credentialsVpsId, setCredentialsVpsId] = useState<string | null>(null);
  const [upgradeVpsId, setUpgradeVpsId] = useState<string | null>(null);

  const usdBalance = wallet.balances["USD"] ?? 0;

  const handleBuyVps = (planId: string) => {
    const id = cloud.buyVps(planId);
    if (id) {
      setNewVpsId(id);
      setSshUser("");
      setSshPassword("");
    }
  };

  const handleSaveNewVpsCredentials = () => {
    if (newVpsId && sshUser.trim()) {
      cloud.setVpsCredentials(newVpsId, sshUser.trim(), sshPassword);
      setNewVpsId(null);
      setSshUser("");
      setSshPassword("");
    }
  };

  const handleOpenCredentials = (vpsId: string) => {
    const vps = cloud.vpsList.find((v) => v.id === vpsId);
    if (vps) {
      setCredentialsVpsId(vpsId);
      setSshUser(vps.sshUser);
      setSshPassword(vps.sshPassword);
    }
  };

  const handleSaveCredentials = () => {
    if (credentialsVpsId && sshUser.trim()) {
      cloud.setVpsCredentials(credentialsVpsId, sshUser.trim(), sshPassword);
      setCredentialsVpsId(null);
    }
  };

  const handleUpgradeVps = (vpsId: string, newPlanId: string) => {
    if (cloud.upgradeVps(vpsId, newPlanId)) setUpgradeVpsId(null);
  };

  const newVps = newVpsId ? cloud.vpsList.find((v) => v.id === newVpsId) : null;

  return (
    <div className={styles.appWithSidebar}>
      <aside className={styles.sidebar}>
        <div className={styles.sidebarSection}>NullCloud</div>
        <button
          type="button"
          className={`${styles.navItem} ${section === "overview" ? styles.navItemActive : ""}`}
          onClick={() => setSection("overview")}
        >
          <span className={styles.navIcon}>
            <LayoutDashboard size={18} />
          </span>
          Overview
        </button>
        <button
          type="button"
          className={`${styles.navItem} ${section === "shop" ? styles.navItemActive : ""}`}
          onClick={() => setSection("shop")}
        >
          <span className={styles.navIcon}>
            <ShoppingCart size={18} />
          </span>
          Shop
        </button>
        <button
          type="button"
          className={`${styles.navItem} ${section === "vps" ? styles.navItemActive : ""}`}
          onClick={() => setSection("vps")}
        >
          <span className={styles.navIcon}>
            <Server size={18} />
          </span>
          My VPS
        </button>
      </aside>
      <main className={styles.main}>
        {section === "overview" && (
          <>
            <h2 className={styles.mainTitle}>Overview</h2>
            <p className={styles.mainSubtitle}>Your virtual machine and VPS instances.</p>
            <div className={styles.card}>
              <div className={styles.cardTitleRow}>
                <Monitor size={20} className={styles.cardTitleIcon} />
                <span className={styles.cardTitle}>Your machine</span>
              </div>
              <p className={styles.cardDesc}>
                This is the virtual machine running Nulltrace. Upgrade it in the Shop.
              </p>
              <div className={styles.overviewSpecGrid}>
                <div className={styles.overviewSpecItem}>
                  <div className={styles.overviewSpecIconWrap}>
                    <Cpu size={22} className={styles.overviewSpecIconCpu} />
                  </div>
                  <div className={styles.overviewSpecLabel}>CPU</div>
                  <div className={styles.overviewSpecValue}>{cloud.localMachine.cpuCores} cores</div>
                </div>
                <div className={styles.overviewSpecItem}>
                  <div className={styles.overviewSpecIconWrap}>
                    <MemoryStick size={22} className={styles.overviewSpecIconRam} />
                  </div>
                  <div className={styles.overviewSpecLabel}>Memory</div>
                  <div className={styles.overviewSpecValue}>{cloud.localMachine.ramGib} GiB</div>
                </div>
                <div className={styles.overviewSpecItem}>
                  <div className={styles.overviewSpecIconWrap}>
                    <HardDrive size={22} className={styles.overviewSpecIconDisk} />
                  </div>
                  <div className={styles.overviewSpecLabel}>Storage</div>
                  <div className={styles.overviewSpecValue}>{cloud.localMachine.diskTotalGib} GiB</div>
                </div>
                <div className={styles.overviewSpecItem}>
                  <div className={styles.overviewSpecIconWrap}>
                    <Wifi size={22} className={styles.overviewSpecIconInternet} />
                  </div>
                  <div className={styles.overviewSpecLabel}>Internet</div>
                  <div className={styles.overviewSpecValue}>
                    {(() => {
                      const plan = cloud.getInternetPlanById(cloud.localMachine.internetPlanId ?? "basic");
                      return plan ? `${plan.name} · ${plan.speedMbps} Mbps` : "—";
                    })()}
                  </div>
                  {cloud.localMachine.internetPlanNextBilling != null && (
                    <div className={styles.overviewSpecMeta}>
                      Next billing: {formatBillingDate(cloud.localMachine.internetPlanNextBilling)}
                    </div>
                  )}
                </div>
              </div>
            </div>
            <div className={styles.sectionTitle}>
              <Server size={18} className={styles.sectionTitleIcon} />
              <span>VPS instances</span>
            </div>
            {cloud.vpsList.length === 0 ? (
              <p className={styles.emptyState}>No VPS yet. Buy one in the Shop.</p>
            ) : (
              <ul className={styles.vpsList}>
                {cloud.vpsList.map((vps) => {
                  const plan = getPlanById(vps.planId);
                  return (
                    <li key={vps.id} className={styles.vpsCard}>
                      <div className={styles.vpsCardHeader}>
                        <span className={styles.vpsIp}>
                          <Globe size={14} className={styles.vpsCardMetaIcon} />
                          {vps.ip}
                        </span>
                        <span className={styles.vpsPlan}>{plan?.name ?? vps.planId}</span>
                      </div>
                      <div className={styles.vpsMeta}>
                        <span className={styles.vpsMetaItem}>
                          <Calendar size={12} className={styles.vpsCardMetaIcon} />
                          Next billing: {formatBillingDate(vps.nextBillingDate)}
                        </span>
                        {plan && plan.weeklyPriceUsd > 0 && (
                          <span className={styles.vpsMetaItem}>
                            Charged ${plan.weeklyPriceUsd.toFixed(2)}/wk
                          </span>
                        )}
                        {!vps.sshUser ? (
                          <span className={styles.vpsMetaItem}>
                            <KeyRound size={12} className={styles.vpsCardMetaIcon} />
                            SSH not configured
                          </span>
                        ) : null}
                      </div>
                      <div className={styles.vpsActions}>
                        <button
                          type="button"
                          className={styles.btn}
                          onClick={() => handleOpenCredentials(vps.id)}
                        >
                          {vps.sshUser ? "Change SSH" : "Configure SSH"}
                        </button>
                        <button
                          type="button"
                          className={styles.btn}
                          onClick={() => setUpgradeVpsId(upgradeVpsId === vps.id ? null : vps.id)}
                        >
                          Upgrade
                        </button>
                      </div>
                      {upgradeVpsId === vps.id && (
                        <div className={styles.vpsActions} style={{ marginTop: "0.5rem" }}>
                          {cloud.vpsPlans
                            .filter((p) => p.id !== vps.planId)
                            .map((p) => (
                              <button
                                key={p.id}
                                type="button"
                                className={`${styles.btn} ${styles.btnPrimary}`}
                                disabled={usdBalance < p.weeklyPriceUsd}
                                onClick={() => handleUpgradeVps(vps.id, p.id)}
                              >
                                {p.name} — ${p.weeklyPriceUsd}/wk
                              </button>
                            ))}
                        </div>
                      )}
                    </li>
                  );
                })}
              </ul>
            )}
          </>
        )}

        {section === "shop" && (
          <>
            <h2 className={styles.mainTitle}>Shop</h2>
            <p className={styles.mainSubtitle}>Upgrade your machine or rent virtual servers. One-time or weekly billing.</p>

            <div className={styles.shopTabs}>
              <button
                type="button"
                className={`${styles.shopTab} ${shopTab === "machine" ? styles.shopTabActive : ""}`}
                onClick={() => setShopTab("machine")}
              >
                <Monitor size={16} />
                Your machine
              </button>
              <button
                type="button"
                className={`${styles.shopTab} ${shopTab === "vps" ? styles.shopTabActive : ""}`}
                onClick={() => setShopTab("vps")}
              >
                <Server size={16} />
                VPS instances
              </button>
            </div>

            {shopTab === "machine" && (
              <div className={styles.shopTabPanel}>
                <p className={styles.shopTabLead}>One-time upgrades for your local virtual machine.</p>

                <div className={`${styles.shopCategory} ${styles.shopCategoryCpu}`}>
                  <div className={styles.shopCategoryHeader}>
                    <div className={styles.shopCategoryIconWrap}>
                      <Cpu size={20} className={styles.shopCategoryIconCpu} />
                    </div>
                    <div>
                      <h3 className={styles.shopCategoryTitle}>Processor</h3>
                      <p className={styles.shopCategoryDesc}>vCPU cores</p>
                    </div>
                  </div>
                  <div className={styles.productGrid}>
                    {cloud.cpuUpgrades
                      .filter((o) => o.value > cloud.localMachine.cpuCores)
                      .map((o) => (
                        <div key={o.value} className={styles.productCard}>
                          <div className={styles.productCardHeader}>
                            <span className={styles.productCardTitle}>{o.label}</span>
                          </div>
                          <div className={styles.productCardSpecs}>
                            {cloud.localMachine.cpuCores} → {o.value} vCPU
                          </div>
                          <div className={styles.productCardPrice}>
                            {o.priceUsd === 0 ? "Free" : `$${o.priceUsd}`}
                          </div>
                          <div className={styles.productCardMeta}>One-time</div>
                          <button
                            type="button"
                            className={styles.btnBuy}
                            disabled={usdBalance < o.priceUsd}
                            onClick={() => cloud.upgradeLocal({ cpuCores: o.value }, o.priceUsd)}
                          >
                            Add upgrade
                          </button>
                        </div>
                      ))}
                    {cloud.cpuUpgrades.filter((o) => o.value > cloud.localMachine.cpuCores).length === 0 && (
                      <div className={styles.productCard}>
                        <div className={styles.productCardHeader}>
                          <span className={styles.productCardTitle}>Current</span>
                        </div>
                        <div className={styles.productCardSpecs}>Max: {cloud.localMachine.cpuCores} vCPU</div>
                        <div className={styles.productCardPriceMuted}>No upgrade available</div>
                      </div>
                    )}
                  </div>
                </div>

                <div className={`${styles.shopCategory} ${styles.shopCategoryRam}`}>
                  <div className={styles.shopCategoryHeader}>
                    <div className={styles.shopCategoryIconWrap}>
                      <MemoryStick size={20} className={styles.shopCategoryIconRam} />
                    </div>
                    <div>
                      <h3 className={styles.shopCategoryTitle}>Memory</h3>
                      <p className={styles.shopCategoryDesc}>RAM</p>
                    </div>
                  </div>
                  <div className={styles.productGrid}>
                    {cloud.ramUpgrades
                      .filter((o) => o.value > cloud.localMachine.ramGib)
                      .map((o) => (
                        <div key={o.value} className={styles.productCard}>
                          <div className={styles.productCardHeader}>
                            <span className={styles.productCardTitle}>{o.label}</span>
                          </div>
                          <div className={styles.productCardSpecs}>
                            {cloud.localMachine.ramGib} → {o.value} GiB RAM
                          </div>
                          <div className={styles.productCardPrice}>
                            {o.priceUsd === 0 ? "Free" : `$${o.priceUsd}`}
                          </div>
                          <div className={styles.productCardMeta}>One-time</div>
                          <button
                            type="button"
                            className={styles.btnBuy}
                            disabled={usdBalance < o.priceUsd}
                            onClick={() => cloud.upgradeLocal({ ramGib: o.value }, o.priceUsd)}
                          >
                            Add upgrade
                          </button>
                        </div>
                      ))}
                    {cloud.ramUpgrades.filter((o) => o.value > cloud.localMachine.ramGib).length === 0 && (
                      <div className={styles.productCard}>
                        <div className={styles.productCardHeader}>
                          <span className={styles.productCardTitle}>Current</span>
                        </div>
                        <div className={styles.productCardSpecs}>Max: {cloud.localMachine.ramGib} GiB</div>
                        <div className={styles.productCardPriceMuted}>No upgrade available</div>
                      </div>
                    )}
                  </div>
                </div>

                <div className={`${styles.shopCategory} ${styles.shopCategoryDisk}`}>
                  <div className={styles.shopCategoryHeader}>
                    <div className={styles.shopCategoryIconWrap}>
                      <HardDrive size={20} className={styles.shopCategoryIconDisk} />
                    </div>
                    <div>
                      <h3 className={styles.shopCategoryTitle}>Storage</h3>
                      <p className={styles.shopCategoryDesc}>Disk capacity</p>
                    </div>
                  </div>
                  <div className={styles.productGrid}>
                    {cloud.diskUpgrades
                      .filter((o) => o.value > cloud.localMachine.diskTotalGib)
                      .map((o) => (
                        <div key={o.value} className={styles.productCard}>
                          <div className={styles.productCardHeader}>
                            <span className={styles.productCardTitle}>{o.label}</span>
                          </div>
                          <div className={styles.productCardSpecs}>
                            {cloud.localMachine.diskTotalGib} → {o.value} GiB
                          </div>
                          <div className={styles.productCardPrice}>
                            {o.priceUsd === 0 ? "Free" : `$${o.priceUsd}`}
                          </div>
                          <div className={styles.productCardMeta}>One-time</div>
                          <button
                            type="button"
                            className={styles.btnBuy}
                            disabled={usdBalance < o.priceUsd}
                            onClick={() => cloud.upgradeLocal({ diskTotalGib: o.value }, o.priceUsd)}
                          >
                            Add upgrade
                          </button>
                        </div>
                      ))}
                    {cloud.diskUpgrades.filter((o) => o.value > cloud.localMachine.diskTotalGib).length === 0 && (
                      <div className={styles.productCard}>
                        <div className={styles.productCardHeader}>
                          <span className={styles.productCardTitle}>Current</span>
                        </div>
                        <div className={styles.productCardSpecs}>Max: {cloud.localMachine.diskTotalGib} GiB</div>
                        <div className={styles.productCardPriceMuted}>No upgrade available</div>
                      </div>
                    )}
                  </div>
                </div>

                <div className={`${styles.shopCategory} ${styles.shopCategoryInternet}`}>
                  <div className={styles.shopCategoryHeader}>
                    <div className={styles.shopCategoryIconWrap}>
                      <Wifi size={20} className={styles.shopCategoryIconInternet} />
                    </div>
                    <div>
                      <h3 className={styles.shopCategoryTitle}>Internet</h3>
                      <p className={styles.shopCategoryDesc}>Connection speed for downloads</p>
                    </div>
                  </div>
                  <div className={styles.productGrid}>
                    {cloud.internetPlans.map((plan) => {
                      const isCurrent = (cloud.localMachine.internetPlanId ?? "basic") === plan.id;
                      return (
                        <div key={plan.id} className={styles.productCard}>
                          {plan.badge && (
                            <span className={styles.productCardBadge}>{plan.badge}</span>
                          )}
                          <div className={styles.productCardHeader}>
                            <span className={styles.productCardTitle}>{plan.name}</span>
                          </div>
                          <div className={styles.productCardSpecs}>{plan.speedMbps} Mbps</div>
                          <div className={styles.productCardPrice}>
                            {plan.weeklyPriceUsd === 0 ? "Free" : `$${plan.weeklyPriceUsd}`}
                            <span className={styles.productCardPriceUnit}>/wk</span>
                          </div>
                          <div className={styles.productCardMeta}>Weekly billing</div>
                          {isCurrent ? (
                            <span className={styles.productCardCurrent}>Current plan</span>
                          ) : (
                            <>
                              <button
                                type="button"
                                className={styles.btnBuy}
                                disabled={usdBalance < plan.weeklyPriceUsd}
                                onClick={() => cloud.subscribeInternet(plan.id)}
                              >
                                Switch to this plan
                              </button>
                              {plan.weeklyPriceUsd > 0 && usdBalance < plan.weeklyPriceUsd && (
                                <p className={styles.insufficientBalance}>Insufficient balance</p>
                              )}
                            </>
                          )}
                        </div>
                      );
                    })}
                  </div>
                </div>
              </div>
            )}

            {shopTab === "vps" && (
              <div className={styles.shopTabPanel}>
                <p className={styles.shopTabLead}>Rent virtual servers. Billed weekly. SSH access included.</p>
                <div className={styles.productGrid}>
                  {cloud.vpsPlans.map((plan) => (
                    <div key={plan.id} className={styles.productCard}>
                      {plan.badge && (
                        <span className={styles.productCardBadge}>{plan.badge}</span>
                      )}
                      <div className={styles.productCardHeader}>
                        <span className={styles.productCardTitle}>{plan.name}</span>
                      </div>
                      <div className={styles.productCardSpecs}>
                        {plan.cpuCores} vCPU · {plan.ramGib} GiB RAM · {plan.diskGib} GiB SSD
                      </div>
                      <div className={styles.productCardPrice}>
                        ${plan.weeklyPriceUsd}<span className={styles.productCardPriceUnit}>/wk</span>
                      </div>
                      <div className={styles.productCardMeta}>Weekly billing</div>
                      {plan.weeklyPriceUsd > 0 && (
                        <p className={styles.productCardChargeInfo}>
                          You will be charged ${plan.weeklyPriceUsd.toFixed(2)} USD per week.
                        </p>
                      )}
                      <button
                        type="button"
                        className={styles.btnBuy}
                        disabled={usdBalance < plan.weeklyPriceUsd}
                        onClick={() => handleBuyVps(plan.id)}
                      >
                        Subscribe
                      </button>
                      {usdBalance < plan.weeklyPriceUsd && (
                        <p className={styles.insufficientBalance}>Insufficient balance</p>
                      )}
                    </div>
                  ))}
                </div>
              </div>
            )}
          </>
        )}

        {section === "vps" && (
          <>
            <h2 className={styles.mainTitle}>My VPS</h2>
            <p className={styles.mainSubtitle}>Manage SSH credentials and upgrades.</p>
            {cloud.vpsList.length === 0 ? (
              <p className={styles.emptyState}>No VPS yet. Buy one in the Shop.</p>
            ) : (
              <ul className={styles.vpsList}>
                {cloud.vpsList.map((vps) => {
                  const plan = getPlanById(vps.planId);
                  return (
                    <li key={vps.id} className={styles.vpsCard}>
                      <div className={styles.vpsCardHeader}>
                        <span className={styles.vpsIp}>
                          <Globe size={14} className={styles.vpsCardMetaIcon} />
                          {vps.ip}
                        </span>
                        <span className={styles.vpsPlan}>{plan?.name ?? vps.planId}</span>
                      </div>
                      <div className={styles.vpsMeta}>
                        <span className={styles.vpsMetaItem}>
                          <KeyRound size={12} className={styles.vpsCardMetaIcon} />
                          SSH: {vps.sshUser || "—"} {vps.sshPassword ? "••••••••" : "(not set)"}
                        </span>
                        <span className={styles.vpsMetaItem}>
                          <Calendar size={12} className={styles.vpsCardMetaIcon} />
                          Next billing: {formatBillingDate(vps.nextBillingDate)}
                        </span>
                        {plan && plan.weeklyPriceUsd > 0 && (
                          <span className={styles.vpsMetaItem}>
                            Charged ${plan.weeklyPriceUsd.toFixed(2)}/wk
                          </span>
                        )}
                      </div>
                      <div className={styles.vpsActions}>
                        <button
                          type="button"
                          className={styles.btn}
                          onClick={() => handleOpenCredentials(vps.id)}
                        >
                          {vps.sshUser ? "Change credentials" : "Configure SSH"}
                        </button>
                        <button
                          type="button"
                          className={styles.btn}
                          onClick={() => setUpgradeVpsId(upgradeVpsId === vps.id ? null : vps.id)}
                        >
                          Upgrade plan
                        </button>
                      </div>
                      {upgradeVpsId === vps.id && (
                        <div className={styles.vpsActions} style={{ marginTop: "0.5rem" }}>
                          {cloud.vpsPlans
                            .filter((p) => p.id !== vps.planId)
                            .map((p) => (
                              <button
                                key={p.id}
                                type="button"
                                className={`${styles.btn} ${styles.btnPrimary}`}
                                disabled={usdBalance < p.weeklyPriceUsd}
                                onClick={() => handleUpgradeVps(vps.id, p.id)}
                              >
                                {p.name} — ${p.weeklyPriceUsd}/wk
                              </button>
                            ))}
                        </div>
                      )}
                    </li>
                  );
                })}
              </ul>
            )}
          </>
        )}
      </main>

      {/* Modal: new VPS — show IP and set SSH */}
      {newVpsId && newVps && (
        <div className={styles.modalOverlay} onClick={() => setNewVpsId(null)}>
          <div className={styles.modal} onClick={(e) => e.stopPropagation()}>
            <div className={styles.modalTitle}>VPS ready</div>
            <div className={styles.modalSubtitle}>Your server is online. Set SSH credentials to connect.</div>
            <div className={styles.modalIp}>{newVps.ip}</div>
            <div className={styles.formGroup}>
              <label className={styles.formLabel}>SSH username</label>
              <input
                type="text"
                className={styles.formInput}
                value={sshUser}
                onChange={(e) => setSshUser(e.target.value)}
                placeholder="e.g. root"
              />
            </div>
            <div className={styles.formGroup}>
              <label className={styles.formLabel}>SSH password</label>
              <input
                type="password"
                className={styles.formInput}
                value={sshPassword}
                onChange={(e) => setSshPassword(e.target.value)}
                placeholder="Optional"
              />
            </div>
            <div className={styles.modalActions}>
              <button type="button" className={styles.btn} onClick={() => setNewVpsId(null)}>
                Skip
              </button>
              <button
                type="button"
                className={`${styles.btn} ${styles.btnPrimary}`}
                onClick={handleSaveNewVpsCredentials}
                disabled={!sshUser.trim()}
              >
                Save
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Modal: change credentials */}
      {credentialsVpsId && (
        <div className={styles.modalOverlay} onClick={() => setCredentialsVpsId(null)}>
          <div className={styles.modal} onClick={(e) => e.stopPropagation()}>
            <div className={styles.modalTitle}>SSH credentials</div>
            <div className={styles.modalSubtitle}>Configure user and password for SSH connection.</div>
            <div className={styles.formGroup}>
              <label className={styles.formLabel}>Username</label>
              <input
                type="text"
                className={styles.formInput}
                value={sshUser}
                onChange={(e) => setSshUser(e.target.value)}
                placeholder="e.g. root"
              />
            </div>
            <div className={styles.formGroup}>
              <label className={styles.formLabel}>Password</label>
              <input
                type="password"
                className={styles.formInput}
                value={sshPassword}
                onChange={(e) => setSshPassword(e.target.value)}
              />
            </div>
            <div className={styles.modalActions}>
              <button type="button" className={styles.btn} onClick={() => setCredentialsVpsId(null)}>
                Cancel
              </button>
              <button
                type="button"
                className={`${styles.btn} ${styles.btnPrimary}`}
                onClick={handleSaveCredentials}
                disabled={!sshUser.trim()}
              >
                Save
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
