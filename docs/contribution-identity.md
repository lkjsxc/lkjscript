# Contribution identity

The repository owner explicitly requests `lkjsxc <lkjsxc@gmail.com>` for work
performed on their behalf. Use that author and committer identity when the
available authorized client exposes those fields. Do not add a ChatGPT identity
or co-author trailer to owner-delegated work. This records the responsible owner;
it does not conceal the use of automated tools or change evidence requirements.

Some repository API clients use the authenticated account's profile name and do
not expose author or committer overrides. A commit associated with the `lkjsxc`
account can therefore retain `Yoshihito Shinnaka <lkjsxc@gmail.com>` in its raw
object. Report that limitation instead of claiming its raw name was changed.
Never change account settings, credentials or access controls to obtain a display
name. Repository-local configuration is not installed by this document.

## Historical display, not history replacement

The root `.mailmap` maps only the three exact historical name/email pairs
belonging to the owner's delegated work. It also normalizes the owner's existing
profile name. Other contributors, GitHub merge identities and GitHub Actions bots
are not mapped. Do not expand this to a global mapping of a shared email address.

Use Git's mailmap-aware views, for example:

```sh
git log --use-mailmap --format='%h %aN <%aE> | %cN <%cE> %s'
git shortlog --summary --numbered HEAD
```

The raw commit objects, parent links, signatures, tags, release assets and source
identities remain unchanged. Historical commits can still show their original
identity in raw or non-mailmap-aware clients, including some hosted views. This
is not a claim that every GitHub page rewrites historical attribution.

A public-history rewrite would change descendant identities and disrupt existing
clones and source-bound evidence. Do not force-update branches or tags merely to
change a displayed name. Preserve truthful historical execution records even
when the display alias is normalized.

The [owner-directed attribution campaign](campaigns/202609241627.md) records the
request, the exact matched identities, independent display tests and the observed
execution boundary. Git's maintained contract is documented in
[gitmailmap](https://git-scm.com/docs/gitmailmap).
