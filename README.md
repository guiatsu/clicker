# Rust AutoClicker

Aplicativo desktop nativo em Rust com interface egui, perfis, atalhos globais, captura de coordenadas e sequências de posições. A aplicação não começa a clicar ao abrir.

## Estado do projeto

O código contém a UI, persistência JSON, modelos/validações e motor cancelável com backend `SendInput` para Windows. O overlay usa uma janela Win32 topmost, transparente e sem interação com o mouse, cobrindo o espaço virtual dos monitores.

Este checkout está sendo trabalhado em WSL2, sem acesso funcional ao shell Windows. O workflow `Windows build` compila para `x86_64-pc-windows-msvc`, executa os checks nativos e publica `RustAutoClicker-windows-x64` como artefato do GitHub Actions. Baixe-o em **Actions → Windows build → Artifacts** após uma execução bem-sucedida. Os testes manuais de hotkeys, overlay, DPI e monitores ainda exigem um desktop Windows.

## Uso

- `F6`: iniciar/parar por alternância.
- `F7`: capturar a posição atual do cursor.
- Os atalhos podem ser alterados na interface e aceitam Ctrl, Shift, Alt e Win.
- Escolha entre cursor atual, posição fixa e sequência ordenada.
- O botão de parada de emergência e o atalho de execução interrompem o motor.

Se o aplicativo não puder criar ou gravar os arquivos de configuração, ele permanece aberto com o perfil padrão e mostra um aviso. Perfis ficam em `%APPDATA%/RustAutoClicker/profiles.json` no Windows; no Linux, em `$XDG_CONFIG_HOME/RustAutoClicker/profiles.json` ou `~/.config/RustAutoClicker/profiles.json`.

## Compilação

Para desenvolvimento no Windows, instale Rust estável, o target MSVC e os componentes C++/Windows SDK conforme a documentação oficial do Rust para Windows. Em seguida:

```powershell
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release --target x86_64-pc-windows-msvc
```

O executável fica em `target/x86_64-pc-windows-msvc/release/RustAutoClicker.exe`. O toolchain Rust estático não exige Rust no computador de destino; instalações Windows podem exigir o runtime Microsoft Visual C++ dependendo do equipamento. Verifique em um Windows limpo antes de distribuir.

O modo Normal usa esperas do scheduler do sistema. Alta precisão faz espera ativa apenas no trecho final de até 250 µs, com maior uso de CPU. Sem limite remove a pausa configurada, mas não remove o tempo de pressionamento, a latência do sistema ou o tempo de processamento do aplicativo receptor. CPS exibido mede eventos concluídos pelo motor e não comprova que outro aplicativo os recebeu.

`SendInput` respeita os limites de integridade do Windows; aplicativos elevados podem rejeitar eventos enviados por um processo comum. O programa não tenta contornar esses limites, anticheat ou outras proteções.

