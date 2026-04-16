<p align="center">
  <img src="https://raw.githubusercontent.com/devscafecommunity/mimi/refs/heads/main/mimi.jpg" alt="MiMi OS Cognitive Architecture">
</p>

# 🤖 MiMi: Multimodal Instruction Master Interface

**MiMi** (Modular Integrated Memory Instance) é um sistema operacional cognitivo de alto desempenho projetado para ser um agente autônomo, expansível e hiper-eficiente. Construída sobre uma espinha dorsal híbrida de **Rust** e **C++**, a MiMi utiliza uma arquitetura de microagentes especializados que se comunicam via Message Bus, garantindo latência mínima e segurança máxima.

Ao contrário de agentes convencionais, a MiMi não apenas executa tarefas; ela **evolui**, criando suas próprias ferramentas (Skills) e gerenciando uma memória de longo prazo baseada em grafos térmicos.

---

## 🏗️ Arquitetura do Ecossistema

O sistema é dividido em núcleos de responsabilidade (Módulos), cada um operando de forma independente e resiliente:

* **Mimi (Agentic Commander):** O orquestrador central escrito em **Rust**. Responsável pela lógica de estado, priorização de tarefas e roteamento de mensagens.
* **Beatrice (NPL Interface):** A ponte humano-máquina. Converte linguagem natural em intenções estruturadas e gerencia o fluxo de I/O.
* **Pandora (ST&LT Memory Manager):** Motor de memória em **C++** integrado ao **Neo4j**. Utiliza algoritmos de *Heatmap* para busca contextual em grafos, reduzindo o consumo de tokens e latência.
* **Echidna (Skills Planner):** O centro de inovação. Analisa repetições e gaps de funcionalidade para criar novas ferramentas (Skills) sob demanda via **Rhai** e **WASM**.
* **Ryzu (Nameless Processors):** Trabalhadores modulares. Subagentes que executam tarefas em ambientes **Docker isolados**, garantindo que o código gerado nunca comprometa o host.
* **Odlaguna (The Moderator):** O supervisor de segurança e integridade. Realiza auditoria de código, validação de segurança e atua como *Watchdog* para evitar processos infinitos.

---

## 🚀 Diferenciais Técnicos

### 1. Contexto Explosivo com Heatmaps (Pandora)
Diferente do RAG (Retrieval-Augmented Generation) tradicional, a Pandora utiliza um sistema de **decaimento térmico** em nós de grafos.
- Dados acessados frequentemente permanecem "quentes" e prontos para uso imediato.
- Dados irrelevantes esfriam, sendo filtrados automaticamente das consultas para evitar o envenenamento de contexto e economizar tokens.

### 2. Ciclo de Evolução Autônoma (Echidna & Odlaguna)
A MiMi pode expandir suas próprias capacidades:
- **Skills Simples:** Geradas em scripts **Rhai** para automação instantânea.
- **Skills Complexas:** Compiladas para **WebAssembly (WASM)**, oferecendo performance nativa com isolamento total de memória.
- **Validação:** Toda skill passa pelo crivo da Odlaguna antes da implementação.

### 3. Comunicação de Baixa Latência
Utilizamos um **Message Bus (Zenoh/NATS)** com serialização **FlatBuffers (Zero-copy)** para garantir que a troca de informações entre os módulos Rust e C++ ocorra em microssegundos.

### 4. Segurança por Design
- Execução em Sandbox (Docker/WASM).
- Limite de instruções (Fueling) para evitar loops infinitos.
- Auditoria estática de código via Odlaguna.

---

## 🛠️ Stack Tecnológica

* **Core:** Rust (Safety & Concurrency)
* **Memory Engine:** C++ (High-performance Computing)
* **Database:** Neo4j (Graph Context)
* **Scripting/Plugins:** Rhai (Embedded), WASM (Sandboxed Binaries)
* **Communication:** Zenoh / ZeroMQ (Message Bus)
* **Virtualization:** Docker (Isolated Workers)
* **AI Adapters:** Gemini API, Ollama (Local LLM)

---

## 📁 Estrutura do Repositório

```text
├── mimi-commander/      # Orquestrador Central (Rust)
├── pandora-memory/      # Driver de Grafos e Heatmap (C++)
├── beatrice-ui/         # Interface de Usuário e NPL (Rust)
├── echidna-lab/         # Gerador de Skills e Compilador WASM
├── odlaguna-guard/      # Monitor de Segurança e Watchdog (Rust)
├── ryzu-runtime/        # Gerenciador de Containers Docker
├── proto/               # Definições de Mensagens (FlatBuffers)
└── skills/              # Biblioteca de Skills Geradas (.rhai / .wasm)
```

---

## 🚥 Como Iniciar (Em breve)

> **Nota:** Este projeto está em fase de desenvolvimento ativo.

1. **Pré-requisitos:**
   - Rust (última versão estável)
   - Clang/LLVM para C++
   - Docker & Docker Compose
   - Instância ativa do Neo4j

2. **Configuração:**
   ```bash
   git clone https://github.com/seu-usuario/mimi.git
   cd mimi
   cargo build --release
   ```

---

## 📜 Licença

Distribuído sob a licença MIT. Veja `LICENSE` para mais informações.

---

> *"A MiMi não é apenas uma ferramenta, é a infraestrutura de uma inteligência que aprende a construir seu próprio futuro."*
