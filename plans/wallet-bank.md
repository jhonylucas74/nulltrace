# Wallet / Bank — Implementation Plan

> **Como usar este documento:** À medida que cada tarefa for concluída, marque o checkbox correspondente trocando `[ ]` por `[x]`. Este documento é a fonte de verdade do progresso da feature.

---

## Contexto

O jogo já possui uma UI de Wallet completa no frontend (`WalletApp.tsx`, ~922 linhas) com seções de Overview, Statement, Transfer, Keys, Card, Convert e NFTs — tudo alimentado por **dados mock** (`lib/wallet*.ts`). O objetivo deste plano é:

1. Criar o schema de banco de dados real para gerenciar saldos, transações, chaves, cartões e faturas.
2. Implementar os serviços backend em Rust que persistem e operam esses dados.
3. Expor as operações via gRPC (já usado no restante do projeto).
4. Conectar o frontend ao backend, substituindo os mocks.
5. Remover completamente a feature de NFTs (UI + dados mock).

O banco é temático: chama-se **Fkebank**. Opera como um banco real — USD tem chave estilo PIX, criptos têm endereços seguindo seus padrões reais.

---

## Referências de Arquivos Críticos

| Área | Arquivo |
|------|---------|
| UI principal da wallet | `nulltrace-client/src/components/WalletApp.tsx` |
| Estado da wallet (Context) | `nulltrace-client/src/contexts/WalletContext.tsx` |
| Mock de saldos | `nulltrace-client/src/lib/walletBalances.ts` |
| Mock de transações | `nulltrace-client/src/lib/walletTransactions.ts` |
| Mock de cartões | `nulltrace-client/src/lib/walletCards.ts` |
| Mock de chaves | `nulltrace-client/src/lib/walletKeys.ts` |
| Mock de conversão | `nulltrace-client/src/lib/walletConversion.ts` |
| Mock de NFTs (remover) | `nulltrace-client/src/lib/walletNfts.ts` |
| Migrations existentes | `nulltrace-core/migrations/001_*.sql` → `015_*.sql` |
| Serviço de players | `nulltrace-core/src/cluster/db/player_service.rs` |

---

## Fase 1 — Banco de Dados (Migrations)

> Próxima migration disponível: `016_*`. Criar uma migration por tabela para facilitar rollback.

### 1.1 — Contas da Wallet (saldos)

```
Migration: 016_create_wallet_accounts.sql
```

- [x] Criar tabela `wallet_accounts`
  - `id UUID PRIMARY KEY DEFAULT gen_random_uuid()`
  - `player_id UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE`
  - `currency VARCHAR(10) NOT NULL` — valores: `'USD'`, `'BTC'`, `'ETH'`, `'SOL'`
  - `balance BIGINT NOT NULL DEFAULT 0` — valor em centavos/satoshis (int); dividir por 100 no frontend para exibição
  - `created_at TIMESTAMPTZ NOT NULL DEFAULT now()`
  - `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()`
  - `UNIQUE(player_id, currency)`
- [x] Seed automático: ao criar player, inserir 4 linhas (uma por currency) com `balance = 0`

---

### 1.2 — Chaves / Endereços de Recebimento

```
Migration: 017_create_wallet_keys.sql
```

- [x] Criar tabela `wallet_keys`
  - `id UUID PRIMARY KEY DEFAULT gen_random_uuid()`
  - `player_id UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE`
  - `currency VARCHAR(10) NOT NULL`
  - `key_address TEXT NOT NULL UNIQUE`
  - `created_at TIMESTAMPTZ NOT NULL DEFAULT now()`
  - `UNIQUE(player_id, currency)` — uma chave por moeda por player

**Formato das chaves (gerado no backend):**

| Moeda | Formato | Exemplo |
|-------|---------|---------|
| USD | `fkebank-{32 hex chars}` | `fkebank-a7b2c9d4e1f6...` |
| BTC | Bech32 simulado: `bc1q{38 alphanumeric}` | `bc1qzv3k9xy...` |
| ETH | `0x{40 hex chars}` | `0x7f3a9b2c1e4d...` |
| SOL | Base58 simulado, 44 chars | `5FHwkrdxktJv...` |

- [x] Seed automático: ao criar player, gerar e inserir uma chave por currency

---

### 1.3 — Transações Gerais

```
Migration: 018_create_wallet_transactions.sql
```

- [x] Criar tabela `wallet_transactions`
  - `id UUID PRIMARY KEY DEFAULT gen_random_uuid()`
  - `player_id UUID NOT NULL REFERENCES players(id)`
  - `type VARCHAR(20) NOT NULL` — `'credit'`, `'debit'`, `'transfer_in'`, `'transfer_out'`, `'convert'`
  - `currency VARCHAR(10) NOT NULL`
  - `amount BIGINT NOT NULL` — em centavos/satoshis (int)
  - `fee BIGINT NOT NULL DEFAULT 0` — em centavos/satoshis (int)
  - `description TEXT`
  - `counterpart_address TEXT` — endereço de destino/origem
  - `counterpart_player_id UUID REFERENCES players(id)` — se for entre players do jogo
  - `related_transaction_id UUID REFERENCES wallet_transactions(id)` — link entre os dois lados de uma transfer
  - `created_at TIMESTAMPTZ NOT NULL DEFAULT now()`
- [x] Index em `(player_id, created_at DESC)` para filtros de statement

---

### 1.4 — Cartões de Crédito

```
Migration: 019_create_wallet_cards.sql
```

- [x] Criar tabela `wallet_cards`
  - `id UUID PRIMARY KEY DEFAULT gen_random_uuid()`
  - `player_id UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE`
  - `label VARCHAR(100)` — nome do cartão ("Main", "Virtual #1", etc.)
  - `number_full VARCHAR(16) NOT NULL` — número completo (para display no app)
  - `last4 VARCHAR(4) NOT NULL`
  - `expiry_month INT NOT NULL`
  - `expiry_year INT NOT NULL`
  - `cvv VARCHAR(4) NOT NULL`
  - `holder_name TEXT NOT NULL`
  - `credit_limit BIGINT NOT NULL DEFAULT 100000` — em centavos (100000 = $1.000,00)
  - `current_debt BIGINT NOT NULL DEFAULT 0` — em centavos
  - `is_virtual BOOLEAN NOT NULL DEFAULT TRUE`
  - `is_active BOOLEAN NOT NULL DEFAULT TRUE`
  - `billing_day_of_week INT NOT NULL DEFAULT 1` — 1 = Monday (ISO)
  - `created_at TIMESTAMPTZ NOT NULL DEFAULT now()`

---

### 1.5 — Transações de Cartão

```
Migration: 020_create_wallet_card_transactions.sql
```

- [x] Criar tabela `wallet_card_transactions`
  - `id UUID PRIMARY KEY DEFAULT gen_random_uuid()`
  - `card_id UUID NOT NULL REFERENCES wallet_cards(id) ON DELETE CASCADE`
  - `player_id UUID NOT NULL REFERENCES players(id)`
  - `type VARCHAR(20) NOT NULL` — `'purchase'`, `'payment'`, `'refund'`
  - `amount BIGINT NOT NULL` — em centavos
  - `description TEXT`
  - `created_at TIMESTAMPTZ NOT NULL DEFAULT now()`
- [x] Index em `(card_id, created_at DESC)`

---

### 1.6 — Faturas Semanais do Cartão

```
Migration: 021_create_wallet_card_statements.sql
```

- [x] Criar tabela `wallet_card_statements`
  - `id UUID PRIMARY KEY DEFAULT gen_random_uuid()`
  - `card_id UUID NOT NULL REFERENCES wallet_cards(id) ON DELETE CASCADE`
  - `period_start TIMESTAMPTZ NOT NULL`
  - `period_end TIMESTAMPTZ NOT NULL`
  - `total_amount BIGINT NOT NULL DEFAULT 0` — em centavos
  - `status VARCHAR(20) NOT NULL DEFAULT 'open'` — `'open'`, `'closed'`, `'paid'`
  - `due_date TIMESTAMPTZ NOT NULL` — próxima segunda-feira após fechamento
  - `paid_at TIMESTAMPTZ`
  - `created_at TIMESTAMPTZ NOT NULL DEFAULT now()`

---

## Fase 2 — Backend Services (Rust)

> Localização: `nulltrace-core/src/cluster/db/`
> Padrão: seguir `email_service.rs` como referência de estrutura.

### 2.1 — `wallet_service.rs`

- [x] Criar arquivo `wallet_service.rs`
- [x] `create_wallet_for_player(player_id)` — cria accounts + keys para todas as currencies
- [x] `get_balances(player_id)` → `Vec<WalletBalance>`
- [x] `get_transactions(player_id, filter: DateFilter)` → filtros: `'today'`, `'7d'`, `'30d'`, `'all'`
- [x] `credit(player_id, currency, amount, description)` — adiciona saldo
- [x] `debit(player_id, currency, amount, description)` — remove saldo com validação de saldo suficiente
- [x] `transfer_to_address(player_id, target_address, currency, amount)` — transfer para endereço externo
- [x] `transfer_between_players(from_player_id, to_player_id, currency, amount)` — transfer interna (atômica via SQL transaction)
- [x] `convert(player_id, from_currency, to_currency, amount)` — conversão com taxa de referência
- [x] `get_keys(player_id)` → endereços por currency
- [x] Integrar `create_wallet_for_player` dentro de `player_service.rs::create_player()`

---

### 2.2 — `wallet_card_service.rs`

- [x] Criar arquivo `wallet_card_service.rs`
- [x] `get_cards(player_id)` → lista de cartões ativos do player
- [x] `create_card(player_id, label, limit)` → gera número, CVV e expiração aleatórios
- [x] `delete_card(card_id, player_id)` — soft delete (`is_active = false`)
- [x] `get_card_transactions(card_id, filter: DateFilter)`
- [x] `make_purchase(card_id, amount, description)` — valida limite, incrementa `current_debt`
- [x] `pay_card_bill(card_id, player_id)` — debita do saldo USD do player, zera `current_debt`
- [x] `get_current_statement(card_id)` — fatura em aberto
- [x] `close_weekly_statement(card_id)` — fecha fatura e abre nova (chamar toda segunda)

---

### 2.3 — Funções Auxiliares de Geração

- [x] `generate_fkebank_key()` → `fkebank-{32 hex chars}`
- [x] `generate_btc_address()` → `bc1q{38 random alphanumeric}`
- [x] `generate_eth_address()` → `0x{40 random hex}`
- [x] `generate_sol_address()` → Base58 de 44 chars
- [x] `generate_card_number()` → 16 dígitos, começa com `4` (Visa)
- [x] `generate_cvv()` → 3 dígitos aleatórios
- [x] `next_billing_monday(from: DateTime)` → próxima segunda-feira

---

## Fase 3 — gRPC / Protocol Buffers

> Localizar o arquivo `.proto` principal em `nulltrace-core/proto/`.

- [x] Adicionar mensagens: `WalletBalance`, `WalletTransaction`, `WalletCard`, `WalletStatement`, `WalletKeys`
- [x] Adicionar RPCs:
  - `GetWalletBalances(PlayerIdRequest) → WalletBalancesResponse`
  - `GetWalletTransactions(WalletTransactionsRequest) → WalletTransactionsResponse`
  - `GetWalletKeys(PlayerIdRequest) → WalletKeysResponse`
  - `TransferFunds(TransferRequest) → TransactionResponse`
  - `ConvertFunds(ConvertRequest) → TransactionResponse`
  - `GetWalletCards(PlayerIdRequest) → WalletCardsResponse`
  - `CreateWalletCard(CreateCardRequest) → WalletCard`
  - `DeleteWalletCard(CardIdRequest) → EmptyResponse`
  - `GetCardTransactions(CardTransactionsRequest) → CardTransactionsResponse`
  - `GetCardStatement(CardIdRequest) → WalletStatement`
  - `PayCardBill(CardIdRequest) → TransactionResponse`
- [x] Gerar código Rust a partir do proto (`build.rs` / `tonic-build`)
- [x] Implementar handlers no arquivo de roteamento gRPC do projeto

---

## Fase 4 — Frontend: Conectar ao Backend

> Substituir dados mock por chamadas gRPC reais via Tauri.

### 4.1 — `WalletContext.tsx`

- [x] Remover imports dos arquivos mock (`walletBalances`, `walletTransactions`, `walletCards`, `walletKeys`, `walletNfts`)
- [x] `useEffect` para buscar saldos ao montar (`GetWalletBalances`)
- [x] `transfer()` → chamar `TransferFunds`
- [x] `convert()` → chamar `ConvertFunds`
- [x] `payBill()` → pagar fatura do cartão de crédito
- [x] Buscar transações com filtro de data real
- [x] Buscar chaves reais do player
- [x] Buscar cartões reais

### 4.2 — `WalletApp.tsx`

- [x] **Statement**: filtros (hoje, 7d, 30d, tudo) passando parâmetro real para o backend
- [x] **Keys**: exibir chaves reais retornadas do backend por currency (uma por moeda)
- [x] **Cards**: listar, criar e deletar via API; exibir `current_debt` e `credit_limit` reais
- [x] **Card Statement**: exibir fatura real com transações reais e data de vencimento
- [x] **Transfer**: mostrar erro de saldo insuficiente vindo do backend
- [x] **Convert (Simulator)**: simulação no frontend com taxas de referência fixas

---

## Fase 5 — Remover NFTs

- [x] Remover import de `walletNfts` em `WalletApp.tsx`
- [x] Remover `"nfts"` do tipo `Section` em `WalletApp.tsx`
- [x] Remover botão "NFTs" da sidebar de navegação em `WalletApp.tsx`
- [x] Remover bloco `case 'nfts':` do render switch em `WalletApp.tsx`
- [x] Remover função `NftsSection()` de `WalletApp.tsx`
- [x] Remover import `Image as ImageIcon` (não usado)
- [x] Deletar `nulltrace-client/src/lib/walletNfts.ts`
- [x] Remover chaves de tradução de NFT em `en/apps.json` e `pt-br/apps.json` (nenhuma chave de NFT encontrada nos arquivos de idioma)

---

## Fase 6 — Internacionalização (i18n)

- [x] Adicionar chaves de tradução para textos novos da wallet em `en/wallet.json`
- [x] Adicionar equivalentes em `pt-br/wallet.json`
- [x] Chaves em uso: `card_limit`, `card_debt`, `card_due_date`, `card_pay_bill`, `card_new_virtual`, `keys_copy`, `transfer_insufficient_balance` (namespace `wallet`)

---

## Fase 7 — Verificação

- [x] Criar player de teste → verificar 4 `wallet_accounts` + 4 `wallet_keys` criados (teste: `test_create_wallet_for_player_creates_accounts_and_keys`)
- [x] Testar `credit` e `debit` via gRPC (testes: `test_credit_increases_balance_and_creates_transaction`, `test_debit_*`)
- [x] Testar transfer entre dois players — verificar saldo de ambos atualizado atomicamente (`test_transfer_between_players_atomic`)
- [x] Testar transfer para endereço externo (sem player destino) (`test_transfer_to_external_address`)
- [x] Testar conversão USD → BTC e verificar saldos corretos (`test_convert_usd_to_btc`, `test_convert_insufficient_balance_fails`)
- [x] Testar criação de cartão virtual (`test_create_card_creates_card_and_open_statement`, `test_get_cards_returns_only_active`)
- [x] Testar compra no cartão → `current_debt` atualizado, erro ao ultrapassar limite (`test_make_purchase_*`, `test_make_purchase_over_limit_fails`)
- [x] Testar pagamento da fatura → débito em USD, `current_debt` zerado (`test_pay_card_bill_*`)
- [x] Testar filtros de statement (hoje, 7d, 30d) (`test_get_transactions_filter_today`, `test_get_card_transactions_filter`)
- [x] Verificar que NFTs foram removidos sem erros de TypeScript
- [x] Verificar formatos de endereço gerados por currency (`test_key_formats`)

---

## Notas de Implementação

- **Atomicidade**: Transfers entre players devem usar `BEGIN/COMMIT` SQL para garantir consistência.
- **Valores monetários como inteiro**: **Todos** os valores monetários são armazenados como `BIGINT` em centavos (ex: $10,50 → `1050`). O frontend é responsável por sempre dividir por `100` antes de exibir (`value / 100`). Nunca usar `FLOAT` ou `NUMERIC` para dinheiro no banco.
- **Fatura semanal**: Fechamento pode ser acionado por Tokio `interval` toda segunda-feira no backend, ou lazily ao abrir a seção de cartão.
- **Segurança**: Validar `player_id` em todas as operações de cartão e wallet. Nunca confiar em `player_id` vindo do cliente.
- **Simulador de conversão**: 100% frontend com taxas fixas de referência — não requer integração de API externa.
