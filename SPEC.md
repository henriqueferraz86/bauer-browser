# Bauer Browser — Especificação
> Versão 0.3 | 2026-06-03 | Metodologia: Spec Driven Development

Status de cada item:
- `[x]` implementado e funcionando
- `[ ]` não implementado ainda
- `[!]` implementado mas quebrado / incompleto

---

## 1. Visão do Produto

O Bauer Browser é um navegador web leve, controlado e seguro, projetado para rodar no BauerOS e em máquinas com recursos limitados. Não compete com Chrome, Edge ou Firefox em compatibilidade máxima — compete em eficiência, controle e integração com o ecossistema Bauer.

**Proposta de valor:**
- Roda bem em máquinas fracas (≤ 4 GB RAM, CPU antiga)
- Integra-se nativamente com o Bauer Agent (IA local)
- Dá ao usuário e ao administrador controle real sobre consumo de recursos
- Protege por padrão: sem trackers, sem autoplay, sem scripts pesados
- Útil para empresas, escolas, ambientes controlados e BauerOS

**Decisões de arquitetura:**
- **Engine:** Wry (usa WebView nativa do OS — WebView2 no Windows, WKWebView no Mac, WebKitGTK no Linux). Zero engine bundled, binário final ~5–15 MB.
- **Linguagem:** Rust. Wry é uma crate Rust, sem binding Python estável. Binário nativo sem runtime.
- **Stack:** `tao` (windowing) + `wry` (WebView) + `serde_json` (IPC) + `sysinfo` (RAM) + `reqwest` (HTTP).

---

## 2. Janela e Layout

| # | Spec | Status |
|---|------|--------|
| 1.1 | Abre janela 1280×800 com título "Bauer Browser" | `[x]` |
| 1.2 | Os 92 px superiores são o chrome WebView (tab bar + toolbar) | `[x]` |
| 1.3 | O content WebView preenche a área abaixo do chrome, borda a borda | `[x]` |
| 1.4 | Redimensionar a janela reajusta chrome e content | `[x]` |
| 1.5 | Título da janela atualiza para `<título da página> — Bauer Browser` | `[x]` |

---

## 3. Navegação

| # | Spec | Status |
|---|------|--------|
| 2.1 | Enter na URL bar navega para o endereço digitado | `[x]` |
| 2.2 | Domínio sem prefixo (ex.: `github.com`) recebe `https://` | `[x]` |
| 2.3 | Texto sem ponto vira busca no DuckDuckGo | `[x]` |
| 2.4 | URL bar vazia navega para DuckDuckGo | `[x]` |
| 2.5 | Botão Voltar dispara `history.back()` | `[x]` |
| 2.6 | Botão Avançar dispara `history.forward()` | `[x]` |
| 2.7 | Botão Recarregar dispara `location.reload()` | `[x]` |
| 2.8 | URL bar atualiza quando a aba ativa navega (incluindo redirecionamentos) | `[x]` |
| 2.9 | Ctrl+L foca a URL bar | `[x]` |
| 2.10 | Alt+← / Alt+→ disparam Voltar / Avançar | `[x]` |
| 2.11 | F5 / Ctrl+R disparam Recarregar | `[x]` |

---

## 4. Abas

| # | Spec | Status |
|---|------|--------|
| 3.1 | Browser abre com uma aba carregada em `home_url` | `[x]` |
| 3.2 | Clicar `+` ou Ctrl+T abre nova aba em `home_url` | `[x]` |
| 3.3 | Nova aba é selecionada automaticamente | `[x]` |
| 3.4 | Clicar em uma aba muda para ela | `[x]` |
| 3.5 | Mudar de aba atualiza a URL bar para a última URL da aba | `[x]` |
| 3.6 | Mudar de aba atualiza o dropdown de modo | `[x]` |
| 3.7 | Clicar `×` em uma aba fecha ela | `[x]` |
| 3.8 | Ctrl+W fecha a aba ativa | `[x]` |
| 3.9 | Fechar uma aba seleciona a aba adjacente, não a última | `[x]` |
| 3.10 | Fechar a última aba abre uma aba nova em branco (não sai do browser) | `[x]` |
| 3.11 | Quando o máximo de abas está aberto, `+` fica visualmente desabilitado | `[x]` |
| 3.12 | Label da aba mostra o título da página; fallback para a URL | `[x]` |
| 3.13 | Máximo de abas lido de `config/settings.toml` (`browser.max_tabs`) | `[x]` |

---

## 5. Modos de Página

| # | Spec | Status |
|---|------|--------|
| 4.1 | Três modos disponíveis: Lite, Normal, Full | `[x]` |
| 4.2 | Modo padrão lido de `config/settings.toml` | `[x]` |
| 4.3 | Lite: bloqueia autoplay ao carregar e em novos elementos de mídia | `[x]` |
| 4.4 | Lite: bloqueia injeção de scripts de terceiros via patch de `createElement` | `[x]` |
| 4.5 | Normal: bloqueia apenas autoplay | `[x]` |
| 4.6 | Full: sem restrições de script | `[x]` |
| 4.7 | Trocar modo aplica a política imediatamente na aba ativa | `[x]` |
| 4.8 | Cada aba lembra seu próprio modo; trocar de aba restaura o modo | `[x]` |
| 4.9 | Botão Reader Mode extrai conteúdo do artigo e reescreve a página | `[x]` |

Tabela de políticas por modo:

| Política | Lite | Normal | Full |
|---|---|---|---|
| Bloqueio de trackers | ✅ | ✅ | ⚠️ Opcional |
| Bloqueio de anúncios | ✅ | ✅ | ⚠️ Opcional |
| Bloquear autoplay | ✅ | ✅ | ❌ |
| Bloquear scripts 3rd party | ✅ | ❌ | ❌ |
| Reader Mode automático | ✅ Quando possível | ❌ Manual | ❌ Manual |

---

## 6. Bloqueio de Ads / Trackers

| # | Spec | Status |
|---|------|--------|
| 5.1 | Navegação para domínio bloqueado é cancelada | `[x]` |
| 5.2 | Lista carregada de `assets/blocklists/tracker_domains.txt` | `[x]` |
| 5.3 | Lista injetada em cada content WebView como `__BAUER_BLOCKED__` | `[x]` |
| 5.4 | Flags `block_trackers` e `block_ads` no config controlam o bloqueio | `[x]` |

---

## 7. Bauer Agent

Fluxo do botão "Resumir":
```
1. Usuário clica "Resumir"
2. Browser extrai texto da página via JS (document.body.innerText)
3. POST {agent_base_url}/summarize  →  { "url": "...", "content": "...", "mode": "summary" }
4. Agente retorna  →  { "summary": "..." }
5. Browser exibe no painel lateral (bottom-right overlay)
```

| # | Spec | Status |
|---|------|--------|
| 6.1 | Botão "Resumir" dispara POST para `agent_base_url/summarize` | `[x]` |
| 6.2 | Botão mostra `⏳…` e fica desabilitado enquanto aguarda | `[x]` |
| 6.3 | Resultado aparece no painel do agente (overlay bottom-right) | `[x]` |
| 6.4 | Painel pode ser fechado com `✕` | `[x]` |
| 6.5 | Se agente inacessível, painel mostra erro com a base URL | `[x]` |
| 6.6 | URL atual da página é enviada ao agente | `[x]` |
| 6.7 | Texto da página (até 8.000 chars) é enviado ao agente | `[x]` |
| 6.8 | `log_agent_request` é chamada antes de cada request HTTP | `[x]` |

Configuração em `settings.toml`:
```toml
[agent]
enabled = true
base_url = "http://localhost:8742"
timeout_seconds = 10
```

---

## 8. Monitor de RAM

| # | Spec | Status |
|---|------|--------|
| 7.1 | Uso de RAM atualiza a cada 2 segundos na toolbar | `[x]` |
| 7.2 | Display fica vermelho quando ultrapassa `ram_alert_mb` | `[x]` |
| 7.3 | Medição inclui processo principal e seus filhos | `[x]` |

---

## 9. Configuração

| # | Spec | Status |
|---|------|--------|
| 8.1 | Todos os settings têm defaults sensatos (sem arquivo de config) | `[x]` |
| 8.2 | `config/settings.toml` é lido na inicialização se existir | `[x]` |
| 8.3 | TOML inválido é reportado no stderr; defaults são usados | `[x]` |

Defaults:
```toml
[browser]
max_tabs     = 5
default_mode = "normal"
home_url     = "https://duckduckgo.com"
ram_alert_mb = 300.0

[blocklists]
block_trackers = true
block_ads      = true

[agent]
enabled         = true
base_url        = "http://localhost:8742"
timeout_seconds = 10

[logging]
enabled = true
```

---

## 10. Logging

| # | Spec | Status |
|---|------|--------|
| 9.1 | Cada navegação é gravada em `~/.bauer-browser/navigation.log` | `[x]` |
| 9.2 | Trocas de modo são logadas | `[x]` |
| 9.3 | Requests ao agente são logados | `[x]` |
| 9.4 | `log_enabled = false` no config desabilita todo o logging | `[x]` |

---

## 11. Requisitos Não Funcionais

| ID | Requisito |
|----|-----------|
| RNF01 | Consumo de RAM do browser sem conteúdo ≤ 100 MB |
| RNF02 | Tempo de inicialização ≤ 3 s em hardware alvo (4 GB RAM, dual-core 2 GHz) |
| RNF03 | Interface responsiva — não travar ao trocar aba ou modo |
| RNF04 | Sem telemetria, sem coleta de dados, sem chamadas de rede desnecessárias |
| RNF05 | Build com um único comando (`cargo build --release`) |

---

## 11b. F-02 — Histórico de Navegação

| # | Spec | Status |
|---|------|--------|
| F-02.1 | Cada URL navegada com sucesso é gravada em `~/.bauer-browser/history.jsonl` | `[x]` |
| F-02.2 | Cada entrada contém: `url`, `title`, `timestamp` | `[x]` |
| F-02.3 | `about:blank` e URLs do sistema não são gravadas | `[x]` |
| F-02.4 | Histórico é carregado na inicialização (máx. 500 entradas mais recentes) | `[x]` |
| F-02.5 | URL bar exibe sugestões de autocomplete do histórico ao digitar | `[x]` |
| F-02.6 | Novas URLs aparecem imediatamente no autocomplete após navegação | `[x]` |

---

## 12. Bugs Conhecidos

Todos os bugs da Fase 0 foram corrigidos. ✓

| ID | Descrição | Status |
|----|-----------|--------|
| B-01 | Títulos das abas — MutationObserver + content IPC adicionados | `[x]` corrigido |
| B-02 | Agent recebia strings vazias — agora extrai URL + innerText via content IPC | `[x]` corrigido |
| B-03 | `log_enabled` ignorado — `AtomicBool` verificado em `write_line` | `[x]` corrigido |
| B-04 | `block_trackers`/`block_ads` ignorados — `block_enabled` wired no navigation handler | `[x]` corrigido |
| B-05 | `log_agent_request` nunca chamada — chamada adicionada em `AgentSummarize` | `[x]` corrigido |
| B-06 | Fechar última aba saía do browser — agora reseta para home | `[x]` corrigido |
| B-07 | Fechar aba selecionava slot errado — lógica adjacente corrigida | `[x]` corrigido |

---

## 13. Roadmap

### Fase 0 — MVP (atual)
Janela funcional, URL bar, modos, bloqueio de navegação top-level, RAM visível, Reader Mode, Bauer Agent básico, logs.

### Fase 1 — Estabilização (pós-MVP)
- **F-01** Favicon nas labels das abas
- **F-03** Favoritos (salvar / listar / abrir)
- **F-04** Download de arquivos
- **F-05** Ícone personalizado do aplicativo
- **F-06** Ctrl+Tab / Ctrl+Shift+Tab para ciclar abas
- **F-07** Listas separadas para trackers vs. ads (conectar ao bug B-04)

### Fase 2 — Corporativo / BauerOS
- Perfis de site (confiável / não confiável)
- Política administrativa via config central
- Integração com BauerOS (tray icon, IPC com Bauer Agent avançado)
- Senhas: integração com keyring do sistema

### Fase 3 — Performance e Portabilidade
- Suporte ARM64 nativo (Raspberry Pi, BauerOS ARM)
- Build reproduzível para Alpine Linux (musl)
- Modo kiosk (WPE WebKit para BauerOS embedded)

---

## 14. Ordem de Implementação Sugerida

1. **B-01** — títulos das abas (maior impacto de UX)
2. **B-06, B-07** — comportamento correto ao fechar aba
3. **B-02** — agente recebe URL + conteúdo reais
4. **B-03, B-05** — respeitar `log_enabled`, chamar `log_agent_request`
5. **B-04** — conectar flags `block_trackers` / `block_ads`
6. **3.11** — desabilitar `+` visualmente quando limite de abas atingido
7. Spec e implementar **F-02** (histórico)
8. Spec e implementar **F-03** (favoritos)
