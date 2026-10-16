# Neo AutoClicker

Aplicativo desktop nativo em Rust para Windows, com perfis, atalhos globais, captura de coordenadas e sequências de posições. Abre em estado parado e não começa a clicar automaticamente.

## Controles

| Ação | Atalho inicial |
|---|---|
| Iniciar / parar | `F6` |
| Capturar ponto sob o cursor | `F7` |
| Mostrar / ocultar marcadores | `F8` |
| Adicionar coordenada | `Ctrl + F9` |
| Limpar pontos (com confirmação) | `Alt + Shift + F9` |

Clique no próprio atalho da interface para gravar outra combinação. A tela mostra cada tecla pressionada; o atalho só é salvo depois que todas forem soltas. Combinações podem conter modificadores ou várias teclas, por exemplo `Ctrl + Shift + F6` e `A + B + C`.

O painel oferece cursor atual, posição fixa e sequência ordenada. CPS e intervalo permanecem sincronizados; também é possível escolher botão e tipo de clique, duração, repetição, ciclos e limite total. A lista de posições aceita captura pelo cursor, coordenadas manuais, reordenação e remoção. O overlay mostra os pontos sobre o desktop sem receber cliques.

## Perfis e dados

Os perfis ficam em `%APPDATA%/NeoAutoClicker/profiles.json` no Windows; no Linux, em `$XDG_CONFIG_HOME/NeoAutoClicker/profiles.json` ou `~/.config/NeoAutoClicker/profiles.json`.

Se a pasta nova ainda não existir, o Neo AutoClicker tenta carregar os dados legados de `RustAutoClicker/profiles.json`. Ao salvar uma alteração, os dados passam para o caminho Neo.

## Desenvolvimento e build

Com Rust estável e toolchain MSVC/Windows SDK instalados:

```powershell
cargo fmt --all -- --check
cargo clippy --all-targets --target x86_64-pc-windows-msvc -- -D warnings
cargo test --target x86_64-pc-windows-msvc
cargo build --release --target x86_64-pc-windows-msvc
```

O executável é `target/x86_64-pc-windows-msvc/release/NeoAutoClicker.exe`. O workflow **Windows build** também publica o artefato `NeoAutoClicker-windows-x64`.

O scheduler usa o intervalo configurado com espera interrompível e um trecho final de espera precisa. Intervalos muito curtos e durações de pressionamento maiores que o intervalo podem reduzir o CPS efetivo; os valores exibidos contam eventos aceitos por `SendInput`, não o que outro aplicativo processou. O programa respeita os limites de integridade do Windows e não tenta contornar anticheat ou outras proteções.
