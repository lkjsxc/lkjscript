# Build a native HTTP service with an ordinary library

This example works with the published Linux x86-64 v0.1.44 executable. It creates
an independently exported response library and a service with a static home page,
health check, query-driven greeting and byte-preserving POST echo. Both programs
are complete native declarations. No compiler checkout, Rust implementation or
host-language semantic generator is needed to author them.

The service uses the existing typed HTTP target and request-scoped `ByteStream`
capability. It does not introduce a second router or a framework-specific intrinsic.
Keep the same executable for both fresh projects so their standard selections agree.

## Create the response library

Start in a new directory with your executable copied to `./lkjscript`. Copy
[http-responses.lkjc](examples/http-responses.lkjc) and
[http-site.lkjc](examples/http-site.lkjc) into that directory, then run:

```sh
./lkjscript new ./responses --template command --name http-responses
./lkjscript --project ./responses status
```

In `http-responses.lkjc`, replace `LIBRARY_BASE` with the `rev_...` from the
`revision id=...` status record. Plan, read the complete logical proposal, then
apply using this plan's own `plan_...` token in place of `LIBRARY_PLAN`:

```sh
./lkjscript --project ./responses change plan --input-file ./http-responses.lkjc --output ./responses.logical-plan
./lkjscript --project ./responses change apply --input-file ./http-responses.lkjc --plan LIBRARY_PLAN
./lkjscript --project ./responses check
./lkjscript --project ./responses package current export --kind transport --output ./responses.lkjp
```

The four exported pure functions are `header`, `respond`, `plain` and
`with-header`. They construct the existing structural response type. `plain`
encodes text as UTF-8; `respond` accepts arbitrary bytes; `with-header` appends
without replacing earlier headers or changing the status/body. These are small
composition helpers, not a validator for every HTTP status/header combination.
They do not add HTML escaping, authentication, redirects or CORS policy.

Keep these fields from the export's `package` record:

| Export field | Placeholder |
| --- | --- |
| `id` | `LIBRARY_PACKAGE` |
| `revision` | `LIBRARY_REVISION` |
| `package-revision` | `LIBRARY_PACKAGE_REVISION` |
| `transport` | `LIBRARY_TRANSPORT` |

The transport carries private implementation bodies and exact dependencies.
Visibility is not source confidentiality. Do not substitute a package name for
these identities or mix transports from independently authored attempts.

## Create the service and import the library

```sh
./lkjscript new ./site --template http --name native-http
./lkjscript --project ./site package dependency stage --transport LIBRARY_TRANSPORT --input-file ./responses.lkjp
./lkjscript --project ./site status
```

Replace `SITE_BASE` in `http-site.lkjc` with the site's current revision and replace
its three library placeholders using the export above. Staging alone does not
change accepted meaning. The request explicitly selects the dependency and creates
the service together. Review and apply its own plan:

```sh
./lkjscript --project ./site change plan --input-file ./http-site.lkjc --output ./site.logical-plan
./lkjscript --project ./site change apply --input-file ./http-site.lkjc --plan SITE_PLAN
./lkjscript --project ./site check
./lkjscript --project ./site build --output ./site/generated/application.lkja
```

The starter targets remain available; the new HTTP target is named `web`.
`GET /greet` uses the first decoded `name` query value. An absent value returns
`Hello, world`, while `name=` returns `Hello, ` with an empty suffix. User input
is served as `text/plain; charset=utf-8`, never interpolated into the static HTML
home page. This is not a general HTML-escaping or template library.

`POST /echo` uses one explicitly allowed `ByteStream.read-all` call with a
1,048,576-byte bound. It returns bytes unchanged as `application/octet-stream`.
The handler's bound and HTTP adapter's body limits are separate controls.

## Select deployment and serve

Copy the generated `site/service.deployment.json` to `site/web.deployment.json`.
Change only the following fields, keeping the generated runtime, execution, stream,
configuration and grant objects intact:

| Field | Value |
| --- | --- |
| `target` | `"web"` |
| `http.maximum_request_body_bytes` | `1048576` |
| `http.maximum_response_body_bytes` | `1048576` |

Keep `artifact` as `"generated/application.lkja"`. The generated `streams` grant
admits the declared request-body operation; use the actual generated authority
revision, not a value copied from another project. Keep the loopback listener
`127.0.0.1:0` for this local example; port zero selects a free port.

```sh
./lkjscript serve --deployment ./site/web.deployment.json
```

Read `local_address` in the `ready` event. In another terminal, substitute that
actual port below:

```sh
ORIGIN=http://127.0.0.1:ACTUAL_PORT
curl -fsS "$ORIGIN/"
curl -i "$ORIGIN/health"
curl -fsS "$ORIGIN/greet?name=Ada&name=Grace"
printf 'native body' | curl -fsS --data-binary @- "$ORIGIN/echo"
```

The health body is `ready` with `cache-control: no-store`; the greeting is
`Hello, Ada`; the echo is `native body`. Stop this foreground server with Ctrl+C.
SIGINT termination was observed to join successfully. Do not assume SIGTERM has
the same graceful behavior in this runner.

Resident HTTP currently requires explicit numeric execution and runtime policy.
Removing `execution` or using `instruction_fuel: null` is not an unmetered mode.
Keep the generated values for this example; its 10,000,000 instruction allowance
is a deployment setting, not a new language requirement introduced by this library.
This guide does not extend foreground optional metering to resident execution.

## Run without the authoring projects

After stopping the first server, copy just the executable, bundle and descriptor:

```sh
mkdir -p ./runtime/generated
cp ./lkjscript ./runtime/lkjscript
cp ./site/generated/application.lkja ./runtime/generated/application.lkja
cp ./site/web.deployment.json ./runtime/web.deployment.json
cd ./runtime
./lkjscript serve --deployment ./web.deployment.json
```

The artifact includes the library and its dependency closure. No library project,
site project, original declaration file or package transport is required by this
runtime. In the recorded test, both authoring roots were moved away from their
original paths before serving; they were retained elsewhere, not destroyed.
The bundle still requires a compatible lkjscript executable. It is not a standalone
machine executable or a replacement official release archive.

## Edit routes through accepted meaning

Return to the authoring directory. Query the target and use its actual `target_...`
identity in place of `WEB_TARGET`:

```sh
./lkjscript --project ./site query find target web
./lkjscript --project ./site change draft --owner WEB_TARGET --output ./web-target.lkjc
./lkjscript --project ./site change plan --input-file ./web-target.lkjc
```

An untouched draft reports `outcome=unchanged`. Find the existing `/health` route's
port reference in that draft. Inside the same target, add the following declaration,
replacing `HEALTH_PORT` with that reference alias while retaining all existing routes:

```text
(route create ready (method GET) (path "/ready") (port HEALTH_PORT))
```

Plan the edited request to a new logical-plan file, review the single added route,
then apply with its new token and check again. Build to a new bundle path and select
it explicitly in a copied deployment descriptor. The new bundle serves both
`/ready` and `/health`; the original bundle still returns 404 for `/ready`.
Changing the draft file alone neither edits accepted meaning nor changes a running
server. A duplicate GET path is rejected at planning without moving accepted HEAD.

## Tested boundary

The public v0.1.44 library check passes 53 graph tests and the service passes 59,
including the exact standard and starter tests: seven library tests and five service
tests are new. Development source `6d2c6df1` passes 59 and 65 respectively because
its standard includes six additional tests. Both evaluators agree. These counts
are not a claim that all tests were authored for this example.

Each executable passed 39 independently asserted detached HTTP cases, including
all 256 octets, exact 1 MiB echo, repeated/empty query values, UTF-8 and NUL,
16 echo requests across eight concurrent clients, declared-size rejection and
recovery. Each also passed eight old/new-bundle selection assertions after the
route edit. This is a designed guide example, not maintained application adoption,
a throughput benchmark or a complete production framework.

For these exact routes, wrong methods (including HEAD), path case changes and a
trailing slash return 404. There is no implicit HEAD-to-GET mapping or automatic
405/Allow behavior in this example. Query data is separate from path matching.
The over-limit test declares Content-Length before uploading; an oversized chunked
body was not included in this acceptance. The listener is plaintext and the guide
does not establish public-network deployment safety, inbound TLS, authentication,
hostile-code isolation or browser-side WebAssembly support.

The [execution record](../campaigns/202609241913.md) retains acquisition boundaries,
failed harness attempts, acceptance scope and the separately unresolved verifier
process-lifecycle defect.
