# Bauer Browser — Especificação SDD
> Versão 0.2 | Data: 2026-06-03 | Metodologia: Spec Driven Development
> **Revisão 0.2:** Mudança de arquitetura — multi-plataforma com Rust + Wry

---

## 1. Visão do Produto

O Bauer Browser é um navegador web leve, controlado e seguro, projetado para rodar no BauerOS e em máquinas com recursos limitados. Ele não compete com Chrome, Edge ou Firefox em compatibilidade máxima — compete em eficiência, controle e integração com o ecossistema Bauer.

**Proposta de valor central:**
- Roda bem em máquinas fracas (≤ 4 GB RAM, CPU de geração antiga)
- Integra-se nativamente com o Bauer Agent (IA local)
- Dá ao usuário e ao administrador controle real sobre consumo de recursos
- Protege por padrão: sem trackers, sem autoplay, sem scripts pesados
- Útil para empresas, escolas, ambientes controlados e BauerOS

---

## 2. Objetivo do MVP

Criar um navegador funcional mínimo que:

- Abre URLs e navega normalmente
- Permite alternar entre modos Lite, Normal e Full
- Mostra consumo de RAM por aba
- Bloqueia autoplay e trackers por padrão
- Tem um botão de resumo de página via Bauer Agent
- Roda localmente no Linux com instruções claras de build e execução
- Não tem extensões, sync, bookmarks avançados ou loja de plugins

**Não é objetivo do MVP:** compatibilidade perfeita com todos os sites. Isso vem nas próximas fases.

---

## 3. Requisitos Funcionais

### Navegação básica
- RF01: Campo de endereço (URL bar) com suporte a HTTPS e HTTP
- RF02: Botões voltar, avançar e recarregar
- RF03: Suporte a abas com limite configurável (padrão: 5 abas)
- RF04: Fechar aba manualmente

### Modos de operação
- RF05: Alternar entre Lite, Normal e Full por aba ou globalmente
- RF06: Aba nova abre no modo padrão configurado
- RF07: Indicador visual do modo ativo em cada aba

### Controle de recursos
- RF08: Exibir uso de RAM aproximado por aba na interface
- RF09: Detectar aba com uso elevado e notificar o usuário
- RF10: Matar aba ociosa após tempo configurável (padrão: desabilitado no MVP)

### Bloqueios (Lite e Normal)
- RF11: Bloquear autoplay de vídeo e áudio por padrão
- RF12: Bloquear trackers conhecidos via lista de domínios bloqueados
- RF13: Bloquear anúncios via lista básica (EasyList simplificada)
- RF14: Bloquear scripts de terceiros pesados no modo Lite

### Modo Leitura
- RF15: Botão para ativar modo leitura simplificado na página atual
- RF16: Modo leitura remove menus, sidebars, ads e deixa só o conteúdo principal

### Integração Bauer Agent
- RF17: Botão "Resumir página" envia conteúdo da página ao Bauer Agent
- RF18: Resposta do agente exibida em painel lateral ou modal simples
- RF19: Interface de chamada ao agente via HTTP local (localhost:PORT)

### Logs e diagnóstico
- RF20: Log básico de navegação (URL, timestamp, modo, uso de RAM)
- RF21: Log exportável em formato texto simples

---

## 4. Requisitos Não Funcionais

- RNF01: Consumo de RAM do próprio browser (sem conteúdo) ≤ 100 MB
- RNF02: Tempo de inicialização ≤ 3 segundos em hardware alvo (4 GB RAM, CPU dual-core 2 GHz)
- RNF03: Interface responsiva — não travar ao trocar de aba ou modo
- RNF04: Código compilável no Alpine Linux e Debian/Ubuntu sem dependências obscuras
- RNF05: Configuração via arquivo de texto simples (INI ou TOML)
- RNF06: Sem telemetria, sem coleta de dados, sem chamadas de rede desnecessárias
- RNF07: Buildável com um único script (`./build.sh` ou `make`)
- RNF08: Documentação suficiente para outro desenvolvedor entender e contribuir

---

## 5. Restrições Técnicas

- RT01: **Sem engine HTML/CSS/JS própria** — usar base pronta (ver seção 7)
- RT02: Alvo primário: Linux x86_64 e ARM64 (Raspberry Pi, etc.)
- RT03: Sem Electron (muito pesado, derrota o propósito)
- RT04: Sem dependência de serviços de nuvem no MVP
- RT05: Bauer Agent deve ser opcional — browser funciona sem ele
- RT06: Interface gráfica via GTK (consistente com BauerOS)
- RT07: Sem store de extensões no MVP — plugins são fase futura

---

## 6. Arquitetura Sugerida

```
┌─────────────────────────────────────────────────────────┐
│                    Bauer Browser UI                      │
│         (GTK4 — Python/Rust, interface principal)        │
│  [URL Bar] [Back/Fwd/Reload] [Tabs] [Mode Switch] [RAM] │
└────────────────────┬────────────────────────────────────┘
                     │
          ┌──────────▼──────────┐
          │   Tab Manager       │  ← controla abas, modo, limites
          │   Resource Monitor  │  ← polling de RAM por processo
          └──────────┬──────────┘
                     │
          ┌──────────▼──────────┐
          │   WebKitGTK WebView │  ← engine de renderização
          │   (por aba)         │
          │   Content Filters   │  ← blocklists (trackers, ads)
          │   Mode Policies     │  ← Lite/Normal/Full via UserScript + ContentFilter
          └──────────┬──────────┘
                     │
          ┌──────────▼──────────┐
          │   Bauer Agent Client│  ← HTTP local, opcional
          │   (page extractor + │
          │    summary request) │
          └─────────────────────┘
```

**Componentes principais:**

| Componente | Responsabilidade |
|---|---|
| UI Layer (GTK4) | Janela, abas, barra de endereço, controles |
| Tab Manager | Ciclo de vida das abas, modo por aba |
| Resource Monitor | Polling de /proc para memória do processo WebKit |
| WebView (WebKitGTK) | Renderização, JS, rede |
| Content Filter Engine | Listas de bloqueio aplicadas ao WebView |
| Mode Policy Engine | Define comportamento do WebView por modo |
| Reader Mode | Extrai e renderiza conteúdo principal da página |
| Bauer Agent Client | Extrai texto da página, envia ao agente, exibe resposta |
| Config Manager | Lê/salva configurações em TOML |
| Logger | Grava log de navegação e recursos |

---

## 7. Escolha da Engine de Renderização

> **Revisão 0.2:** Requisito de multi-plataforma (Windows/Mac/Linux) elimina WebKitGTK.

### Opções avaliadas

| Engine | Peso | Windows | Mac | Linux | Veredicto |
|---|---|---|---|---|---|
| **Wry (nativa do OS)** | Zero bundled | WebView2 (Edge) | WKWebView | WebKitGTK | ✅ **Escolhida** |
| CEF (Chromium) | ~300 MB | ✅ | ✅ | ✅ | ❌ Muito pesado |
| QtWebEngine | ~200 MB | ✅ | ✅ | ✅ | ❌ Pesado, Qt dep |
| WebKitGTK | Leve | ❌ | ❌ | ✅ | ❌ Linux-only |
| cefpython3 | ~300 MB | ✅ | ✅ | ✅ | ❌ Abandonado |

### Decisão: **Wry**

**Justificativa:**
- **Zero engine bundled**: usa a WebView já instalada no sistema operacional
  - Windows 10/11: WebView2 (Microsoft Edge Chromium) — instalado por padrão desde 2021
  - macOS: WKWebView (WebKit/Safari) — nativo do OS
  - Linux: WebKitGTK — disponível via pacote
- Binário Rust final é leve (~5–15 MB sem a engine)
- API Rust unificada que abstrai as diferenças entre plataformas
- Mantido pela equipe do Tauri, ativo e com boa adoção
- Suporta múltiplos WebViews numa mesma janela (chrome + content)
- Suporte a IPC, injeção de JS, filtro de navegação

**Trade-offs aceitos:**
- Comportamento pode variar ligeiramente entre OSes (versões de engine distintas)
- Bloqueio de sub-recursos (ads/trackers) requer APIs específicas de cada plataforma (V2)
- No MVP, bloqueio funciona em nível de navegação (top-level URLs)

---

## 8. Linguagem de Implementação

> **Revisão 0.2:** Rust é obrigatório para Wry (sem bindings Python). Decisão correta para multi-plataforma.

### Decisão: **Rust**

**Justificativa:**
- Wry é uma crate Rust — não existe binding Python estável
- Binário nativo sem runtime: `.exe` no Windows, sem dependência de Python
- Performance e segurança de memória nativamente
- `tao` (windowing) + `wry` (WebView) = stack oficial do ecossistema Tauri
- `serde_json` para IPC entre chrome HTML e Rust
- `sysinfo` para monitoramento de RAM cross-platform
- `reqwest` para chamadas ao Bauer Agent (HTTP async)
- Distribuição simples: único binário por plataforma

**Prototipagem:** os arquivos Python em `_prototype_linux/` documentam o design original Linux-only e servem como referência de lógica de negócio.

---

## 9. Estrutura de Pastas

> **Revisão 0.2:** Projeto Cargo (Rust). Python movido para `_prototype_linux/`.

```
bauer-browser/
├── Cargo.toml               # Dependências Rust
├── Cargo.lock
├── src/
│   ├── main.rs              # Entry point: event loop tao + WebViews wry
│   ├── config.rs            # Leitura de TOML de configuração
│   ├── mode.rs              # Políticas Lite/Normal/Full (JS injection)
│   ├── filter.rs            # Lista de domínios bloqueados
│   ├── resource.rs          # Monitoramento de RAM via sysinfo
│   ├── agent.rs             # HTTP client para Bauer Agent
│   └── logger.rs            # Log de navegação em arquivo
├── chrome/
│   └── index.html           # UI da toolbar (URL bar, botões, modo, RAM)
├── assets/
│   ├── blocklists/
│   │   └── tracker_domains.txt  # Domínios bloqueados (1 por linha)
│   └── reader_mode.css      # CSS do modo leitura
├── config/
│   ├── defaults.toml
│   └── settings.toml        # Configuração do usuário
├── build.bat                # Build Windows
├── build.sh                 # Build Linux/Mac
├── README.md
└── _prototype_linux/        # Protótipo Python original (referência)
    ├── main.py
    └── browser/
```

---

## 10. Estratégia para Controle de RAM/CPU

### RAM

**Monitoramento:**
- Cada aba tem um WebView que roda em processo auxiliar (`WebKitWebProcess`)
- Polling a cada 2 segundos via `/proc/<pid>/status` (campo `VmRSS`) para obter RSS do processo
- PID do processo auxiliar obtido via `WebKitWebView.get_web_process_identifier()`
- Exibição na tab label: `"exemplo.com — 87 MB"`

**Limites:**
- MVP: apenas exibição e alerta visual quando > threshold configurável (padrão: 300 MB por aba)
- V2: usar `RLIMIT_AS` ou cgroups v2 para limite hard (requer mais integração com OS)

### CPU

- MVP: sem limite de CPU (complexidade alta, impacto no usuário)
- Monitoramento: calcular `%CPU` via `/proc/<pid>/stat` entre dois polling intervals
- Alerta visual se aba usar > X% de CPU por mais de Y segundos (configurável)
- V2: cgroups v2 para throttling por aba

### Aba ociosa
- Detectar via `WebKitWebView` sem foco por tempo configurável
- MVP: apenas sinalizar — V2 suspender processo ou liberar memória

---

## 11. Estratégia Lite / Normal / Full

Cada modo é um conjunto de políticas aplicadas ao `WebKitWebView` no momento da criação ou quando o usuário troca de modo (recarrega a aba).

| Política | Lite | Normal | Full |
|---|---|---|---|
| Content Filter (trackers) | ✅ Ativo | ✅ Ativo | ⚠️ Opcional |
| Content Filter (ads) | ✅ Ativo | ✅ Ativo | ⚠️ Opcional |
| Bloquear autoplay | ✅ Sim | ✅ Sim | ❌ Não |
| Bloquear scripts 3rd party | ✅ Sim | ❌ Não | ❌ Não |
| JavaScript | ⚠️ Restrito | ✅ Normal | ✅ Normal |
| Imagens | ✅ Sim | ✅ Sim | ✅ Sim |
| Cookies | Apenas 1st party | Apenas 1st party | Todos |
| Reader Mode auto | ✅ Quando possível | ❌ Manual | ❌ Manual |
| WebGL / Media | ❌ Desabilitado | ⚠️ Limitado | ✅ Completo |

**Implementação técnica:**
- `WebKitSettings` configura JS, autoplay, WebGL por aba
- `WebKitUserContentFilterStore` aplica JSON de regras (Content Blocker API)
- UserScript injeta JS para desabilitar autoplay: `document.querySelectorAll('video, audio').forEach(m => { m.autoplay = false; m.pause(); })`
- Troca de modo: recriar WebView ou recarregar com novas settings (MVP: recarrega)

---

## 12. Estratégia de Integração com Bauer Agent

**Premissa:** Bauer Agent expõe uma API HTTP local (ex.: `http://localhost:8742`)

**Fluxo do botão "Resumir Página":**

```
1. Usuário clica "Resumir" na toolbar
2. browser extrai texto da página via JS:
   document.body.innerText (simplificado)
   ou via reader_mode.extract() para conteúdo limpo
3. POST http://localhost:8742/summarize
   { "url": "...", "content": "...", "mode": "summary" }
4. Bauer Agent processa e retorna JSON:
   { "summary": "..." }
5. Browser exibe summary em sidebar ou modal GTK
```

**Resiliência:**
- Se agente não estiver disponível: botão desabilitado com tooltip "Bauer Agent não conectado"
- Health check a cada 30s via `GET /health`
- Timeout de 10s na requisição

**Configuração** (settings.toml):
```toml
[agent]
enabled = true
base_url = "http://localhost:8742"
timeout_seconds = 10
```

---

## 13. Critérios de Aceite (MVP)

| ID | Critério | Como verificar |
|---|---|---|
| CA01 | Browser abre janela ao executar `./build.sh run` | Visual |
| CA02 | Campo URL aceita endereço e navega ao pressionar Enter | Navegar para example.com |
| CA03 | Botões voltar, avançar e recarregar funcionam | Navegar em 3 páginas |
| CA04 | Máximo de N abas configurável (testar com N=3) | Tentar abrir 4a aba → bloqueado |
| CA05 | Indicador de RAM visível e atualizado a cada 2s | Abrir página pesada e observar |
| CA06 | Trocar para Lite Mode bloqueia autoplay | Abrir YouTube em Lite → vídeo não inicia |
| CA07 | Trocar para Full Mode permite autoplay | Abrir YouTube em Full → vídeo inicia |
| CA08 | Tracker bloqueado no Normal/Lite Mode | Checar log de rede — google-analytics.com bloqueado |
| CA09 | Botão Reader Mode exibe versão limpa da página | Abrir artigo de blog |
| CA10 | Botão "Resumir" envia request ao Bauer Agent | Mock do agente recebe POST |
| CA11 | Sem agente: botão desabilitado com tooltip | Matar processo do agente |
| CA12 | Log de navegação gravado em arquivo | Abrir 3 URLs → verificar log.txt |
| CA13 | `./build.sh` instala dependências e roda sem erro manual | Ambiente limpo |

---

## 14. Roadmap por Fases

### Fase 0 — MVP (atual) — Rust + Wry
Janela funcional, URL bar, modos, bloqueio de navegação (top-level), RAM visível, Reader Mode, Bauer Agent básico, logs. Windows first.

### Fase 1 — Estabilização (pós-MVP)
- Histórico de navegação local
- Favoritos simples
- Aba ociosa: suspender processo após inatividade
- Limite hard de RAM via cgroups v2
- Detecção de página pesada com sugestão de Lite Mode
- Modo leitura mais robusto (Readability.js port)
- Atualização automática das blocklists

### Fase 2 — Corporativo / BauerOS
- Perfis de site (confiável / não confiável)
- Política administrativa via arquivo de config central
- Integração com BauerOS (tray icon, IPC com Bauer Agent avançado)
- Download manager básico
- Senhas: integração com keyring do sistema

### Fase 3 — Performance e Portabilidade
- Reescrever módulos críticos em Rust (resource monitor, content filter)
- Versão WPE WebKit para BauerOS embedded / kiosk
- Suporte ARM64 nativo (Raspberry Pi, BauerOS ARM)
- Build reproduzível para Alpine Linux (musl)

### Fase 4 — Extensibilidade
- API de plugins local (sem store externa)
- Integração com Bauer Agent avançada (comandos de voz, automação)
- Modo multi-perfil (usuário pessoal vs corporativo)

---

## 15. Lista de Tarefas Técnicas para Implementação do MVP (Rust + Wry)

### Setup
- [ ] T01: Criar `Cargo.toml` com dependências wry, tao, serde_json, sysinfo, dirs
- [ ] T02: Criar estrutura de pastas conforme seção 9
- [ ] T03: Criar `build.bat` (Windows) e `build.sh` (Linux/Mac)
- [ ] T04: Criar `config/defaults.toml`

### Core (src/main.rs)
- [ ] T05: Event loop `tao::EventLoop<AppEvent>` com user events
- [ ] T06: Chrome WebView (toolbar HTML, bounds fixo topo)
- [ ] T07: Content WebView (navega URLs externas, bounds abaixo do chrome)
- [ ] T08: Resize handler — atualiza bounds dos dois WebViews
- [ ] T09: IPC handler — recebe comandos JSON do chrome HTML
- [ ] T10: URL sync — content URL change → atualiza input no chrome
- [ ] T11: Título sync — content title change → atualiza window title

### Módulos Rust
- [ ] T12: `src/mode.rs` — injeta JS de autoplay/scripts por modo
- [ ] T13: `src/filter.rs` — carrega tracker_domains.txt, with_navigation_handler
- [ ] T14: `src/resource.rs` — polling RAM via sysinfo, envia update ao chrome
- [ ] T15: `src/agent.rs` — reqwest async POST /summarize + GET /health
- [ ] T16: `src/logger.rs` — grava log em arquivo via dirs
- [ ] T17: `src/config.rs` — lê settings.toml com toml crate

### Chrome UI (chrome/index.html)
- [ ] T18: Toolbar HTML/CSS dark theme
- [ ] T19: Botões back/forward/reload com send() IPC
- [ ] T20: URL input com Enter para navegar
- [ ] T21: Seletor de modo (Lite/Normal/Full)
- [ ] T22: Botão Reader Mode
- [ ] T23: Botão Resumir (Bauer Agent)
- [ ] T24: Display de RAM (atualizado via evaluate_script do Rust)

### Assets e Config
- [ ] T25: `assets/blocklists/tracker_domains.txt` com domínios bloqueados
- [ ] T26: `assets/reader_mode.css`
- [ ] T27: `config/settings.toml` documentado

### Documentação
- [ ] T28: `README.md` com install Rust, WebView2, build, run
- [ ] T29: Mover arquivos Python para `_prototype_linux/`

---

## Pendências para V2 (fora do escopo MVP)

- Limite hard de RAM/CPU via cgroups v2
- Suspensão de aba ociosa
- Detecção automática de página pesada
- Histórico e favoritos
- Modo leitura robusto (Readability.js)
- Perfis de site (confiável / não confiável)
- Políticas administrativas
- Download manager
- Build para Alpine Linux / musl
- Reescrita de módulos críticos em Rust

---

*Fim da Especificação — Bauer Browser v0.1 MVP*
