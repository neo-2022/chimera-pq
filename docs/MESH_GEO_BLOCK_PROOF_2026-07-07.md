# CHIMERA-PQ / WEAVE — реальные проверки обхода геоблокировки

## Общие параметры прогона

- **Версия релиза:** `v0.1.178`
- **Стенд:** два симметричных mesh-узла (primary — NL, secondary — RU).
- **Режим узла:** `node` (симметричный).
- **Split-tunnel env:** `CHIMERA_APPLY_TUN=false`,
  `CHIMERA_APPLY_ROUTE=false`, `CHIMERA_APPLY_DNS=false`.
- **Пользователь:** `chimera-test` (UID `10001`), `nobody` (UID `65534`).
- **Сервисы запущены через systemd --user:**
  - `chimera-runtime.service`
  - `chimera-node.service`
  - `chimera-datapath.service`

## Что чинилось на стенде (runtime-конфиг, не продукт)

После автообновления до `v0.1.178` оба узла стартовали, но mesh между
узлами не поднимался из-за рассинхронизации peer-портов в bootstrap- и
peer-egress-конфигах. Исправлено оператором только в runtime-файлах на
стенде (без ручного копирования бинарей):

- `CHIMERA_PEER_EGRESS_PEER_LISTEN` выставлен в `<redacted-ip>`.
- `CHIMERA_PEER_EGRESS_SERVER` выровнен на удалённый узел `:18142`.
- `CHIMERA_MESH_REMOTE_PEER_SPEC` в `<redacted-path>`
  приведён к тому же порту.
- `<redacted-path>`: `carrier.addr` и
  `peer.listen_addr` зафиксированы на `:18142`.
- После этого оба направления mesh (primary→secondary и secondary→primary)
  установились.

## Повторная проверка split-tunnel UID 10001 / 65534

### Primary (NL) → egress через secondary (RU)

```text
UID 10001  ipinfo.io → country=RU, ip=<secondary-ru-public>
UID 65534  ipinfo.io → country=RU, ip=<secondary-ru-public>
```

### Secondary (RU) → egress через primary (NL)

```text
UID 10001  ipinfo.io → country=NL, ip=<primary-nl-public>
UID 65534  ipinfo.io → country=NL, ip=<primary-nl-public>
```

Вывод: split-tunnel правила захватывают трафик по UID и домену,
прямой root-трафик остаётся на локальном IP, UID 10001/65534 уходят
через mesh на удалённый узел.

## Реальные проверки geo-блока

### Secondary (RU) → foreign сервисы через primary (NL)

`CHIMERA_CAPTURE_DOMAIN=chatgpt.com,chat.openai.com,youtube.com,x.com,twitter.com,gemini.google.com,ipinfo.io`

| URL | HTTP-код | Примечание |
|-----|----------|------------|
| `https://ipinfo.io` | `200` | страна=NL, egress через primary |
| `https://chatgpt.com` | `403` | TCP/TLS через mesh дошли, блок на уровне Cloudflare/datacenter IP |
| `https://youtube.com` | `200` | редирект на www.youtube.com |
| `https://x.com` | `200` | рабочий ответ |
| `https://gemini.google.com` | `200` | рабочий ответ |

> `403` от ChatGPT не означает неработоспособности туннеля — это типичная
> реакция площадки на IP хостинга/датацентра. Соединение прошло через NL
> egress и TLS-рукопожатие завершилось.

### Primary (NL) → РФ-сервисы через secondary (RU)

`CHIMERA_CAPTURE_DOMAIN=rbc.ru,lenta.ru,kp.ru,market.yandex.ru,ria.ru,sberbank.ru,ipinfo.io`

| URL | HTTP-код | Примечание |
|-----|----------|------------|
| `https://ipinfo.io` | `200` | страна=RU, egress через secondary |
| `https://lenta.ru` | `200` | рабочий ответ |
| `https://ria.ru` | `200` | рабочий ответ |
| `https://rbc.ru` | `401` | TCP/TLS через mesh дошли, площадка отвечает ( captcha/anti-bot ) |
| `https://market.yandex.ru` | `200` + `showcaptcha` | соединение через RU, но яндекс отдаёт капчу на DC IP |
| `https://kp.ru` | `303` | соединение через RU |

> Для части крупных российских порталов установление HTTPS-соединения
> и получение HTTP-ответа подтверждают, что трафик действительно выходит
> из России. Коды `401`/капча — защита площадки от датацентровых IP, а
> не неработоспособность туннеля.

## Статус сервисов после прогона

```text
primary:  chimera-runtime=active  chimera-node=active  chimera-datapath=active
secondary: chimera-runtime=active  chimera-node=active  chimera-datapath=active
```

## Ограничения / остатки

- Некоторые популярные foreign площадки (ChatGPT) отдают HTTP-заглушки
  для датацентровых IP; это не связано с CHIMERA, а с IP-репутацией.
- `chimera-sh -start` из обычной shell-сессии по-прежнему требует явного
  экспорта `CHIMERA_APPLY_TUN=false`/`ROUTE`/`DNS`, либо запуска через
  `systemctl --user start chimera-runtime.service`. В `chimera-control.sh`
  добавлено наследование APPLY-флагов из systemd-окружения (local commit
  `901de19`), но новый релиз под это не выкладывался.
- Для стабильной работы симметричного двухузлового стенда peer-порт и
  bootstrap-спецификация должны быть согласованы на обоих концах.
  Автохилинг `v0.1.178` не устраняет рассинхронизацию порта, если
  `mesh_bootstrap.env` содержит устаревший порт.

## Доказательства

- service status: `systemctl --user is-active chimera-runtime.service`
- node log: `<redacted-path>`
- datapath log: `<redacted-path>`
- правила nft: `nft list table inet chimera_redirect`
- capture env: `<redacted-path>`
