# money.null + Account / key-based wallet model

> Plano revisado: Account para USD, identificação por chave/endereço (sem depender de player_id em transações). Inclui VM money.null, crypto (chave privada em arquivos na VM), APIs Lua Fkebank e crypto.

---

## Migrations em dev (podem ser alteradas ou removidas)

**Contexto:** Estamos em desenvolvimento e o BD é apagado com frequência. Não é necessário preservar dados nem fazer migração incremental a partir do schema atual.

**Estratégia:**

- **Podemos remover ou reescrever** as migrations de wallet atuais (016–021).
- **Substituir** por um conjunto novo de migrations que já implementem o modelo Account + key-based desde o início:
  - `fkebank_accounts` (USD: owner_type, owner_id, key, full_name, document_id, balance)
  - `fkebank_tokens` (token por account para autorizar transferências)
  - `crypto_wallets` (key_address, public_key, currency, balance) — **sem owner**: nenhum player_id, vm_id ou owner_type; não rastreáveis
  - Tabela de transações com **from_key** e **to_key** (sem player_id/vm_id nas linhas)
- Em [nulltrace-core/src/cluster/db/mod.rs](nulltrace-core/src/cluster/db/mod.rs), trocar as chamadas que rodam 016–021 pelas novas migrations (ou reutilizar números 016–021 com conteúdo novo, ou usar novos números e remover os arquivos antigos).
- **Cartões** (019–021): decidir se mantemos no novo modelo (ex.: cartão atrelado a `account_id` ou ao player por enquanto) e incluir na nova migration ou em migration separada.

Assim evitamos migração de dados antigos e partimos direto do schema desejado.

---

## Modelo de dados (resumo)

- **USD:** Entidade **Account** (player ou VM). Chave PIX identifica a conta; transações usam from_key / to_key.
- **Crypto:** **Sem dono no banco.** Carteiras crypto não têm player_id, vm_id nem qualquer owner; não são rastreáveis. A existência da carteira se define apenas pelas **chaves** (pública/privada) e pelos **arquivos** (chave privada em disco). A tabela `crypto_wallets` guarda só: endereço/chave pública, currency, saldo. Quem tem o arquivo da chave privada é quem “controla” a carteira; isso não fica registrado no BD. Para gastar, o caller (ex.: Lua na VM) passa o caminho do arquivo da chave privada; o backend lê o arquivo, confere que bate com o endereço, debita esse endereço — sem gravar qual VM ou usuário.
- **Transações:** Sempre from_key, to_key (e counterpart_key se útil). Listagem “minhas transações” = WHERE from_key = my_key OR to_key = my_key.

---

## Lua: histórico de transações

**Crypto:** Permitir via Lua obter o histórico de **qualquer** carteira crypto só com a **chave pública** (endereço). Sem auth: quem tem o endereço pode consultar. Ex.: `crypto.history(address)` retorna transações onde `from_key = address OR to_key = address`.

**USD (Fkebank):** Para o banco, **é obrigatório o token privado** (token de autorização da conta). Não basta ter a chave PIX; é preciso enviar o token (ex.: lido de `/etc/wallet/fkebank/token`) para ver histórico ou fazer operações. Ex.: `fkebank.history(key, token)` — o backend valida o token para essa conta antes de devolver as transações.

(O restante do plano — money.null VM, “return half”, testes — segue o mesmo desenho, com crypto sem dono no schema.)
