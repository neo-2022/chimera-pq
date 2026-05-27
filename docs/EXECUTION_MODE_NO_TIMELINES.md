# CHIMERA-PQ: Режим выполнения без таймлайнов

Статус: active  
Режим работы: execution-by-readiness (без календарных дедлайнов)

## Правило

Для текущего контура CHIMERA-PQ MVP:

- никаких планов с дедлайнами по дням/неделям;
- никакого закрытия этапов по дате;
- прогресс принимается только по доказательствам и gate-критериям;
- блок получает статус `done` только после PASS по proof bundle.

## Модель поставки

Работа ведется функциональными блоками:

1. Стабильность runtime и failover-поведение
2. Согласованность DNS/route и leak-safety
3. Поведение split/full traffic policy
4. Надежность install/start/stop/uninstall
5. Mesh launch и двусторонние runtime-проверки
6. Нагрузочная и long-run устойчивость
7. Release-readiness evidence pack

Для каждого блока действует единый контракт закрытия:

- `Status`: done | partial | not done
- `Evidence`: артефакты + команды + логи
- `Unclosed`: что осталось незакрытым
- `Risks`: остаточные ограничения

## Жесткий gate

Блок нельзя считать закрытым, если отсутствует хотя бы один пункт:

1. acceptance-критерии цели;
2. proof-артефакты по критическим путям;
3. negative-path проверка;
4. non-regression проверки соседних контуров.

## Текущая директива исполнения

- timelines removed;
- выполнение продолжается без паузы;
- приоритет остается на MVP-scope из `CHIMERA-PQ_MVP_SPEC.md`.

## Обязательный закон исполнения команд пользователя

Добавлено по прямому указанию пользователя (2026-05-23).

- запрещен обман;
- запрещена неточность выполнения команды;
- запрещена интерпретация команды;
- запрещена подмена намерения пользователя;
- запрещена остановка на частичной реализации, если запрошено полное завершение.

Override для закрытия блока:

- статус `done/pass` запрещен, пока точный результат команды пользователя не
  подтвержден релевантными real-world проверками (а не только частичными/
  simulated/proxy-only данными).

## Absolute Completion Lock

Жесткая блокировка исполнения:

1. Запрещена промежуточная остановка, пока команда пользователя открыта.
2. Запрещено финальное `done/pass`, пока не завершены исчерпывающие исполнение и проверка.
3. Если любой обязательный чек отсутствует или не пройден, статус только
   `partial` или `not done`.

Обязательный pre-response lock-check:

- `command_exhaustive = true`
- `verification_exhaustive = true`
- `evidence_exhaustive = true`

Если хотя бы одно поле `false`, финальный статус `done/pass` запрещен.

## Invisible UX Enforcement (Mandatory)

Added by explicit user instruction (2026-05-26).

For runtime/app-access blocks, `done/pass` is forbidden unless all conditions hold:

1. target resources are reachable in normal user app workflow;
2. no mandatory app-specific proxy setup is required;
3. no mandatory app relaunch routine is required;
4. evidence is collected from real runtime behavior, not proxy-only workaround mode.

Execution ban:

- closing a block with only manual app-proxy workaround evidence is prohibited;
- closing a block after forcing browser/IDE proxy launch flags is prohibited.
