## Steps to first public release version

* Steps before final release:
  * move to pluralsync.org
  * make bridge auto-updating
  * send both on an announcement email
* Sync from Pluralkit
  * TODO wait for proper system-based rate-limiting in PluraKit
  * TODO have a proper start time per member based on the latest multiple switches here
* fix pluralsync bridge issues:
  * make it auto-updating
  * make each login provide the version of the software sending the request - and only accept the current version
  * once auto-updating, let the users know, that it's auto-updating via email
* move to pluralsync.org for more proper version
* DONE: extract user agent and make them non-public deployment/build vars
* easily give quick feedback in website - whcih will be saved in db and then I can response to it
* suggested by Aino: make general interviews with a few diverse systems. focus on their needs regardless of pluralsync specifically.
  * getting a hollistic/encompassing understanding is very useful. equally also having a high diversity there.
  * it might also make sense to learn a bit more about plural systems in general and how they use social networks.
    * we can do that in interviews or just browse through reddit and a few places on discord for that
      * on discord just read some channels with plural systems
      * and also checkout the inspirations channel on simply plural discord
  * The core focus is to simply listen and be curious without any specific intention of being a pluralsync user.
* pluralsync-bridge
  * auto-update
  * DONE: auto-start on system start
* deploy first proper version
* finalize README
* add database backups

## Feedback from second test phase users
* configureable fronting order (details in discord)
* something like this for the website view as well as the fronting link would be good probably: https://codeberg.org/fulmine/pluralkit-xyz
  * their formatting of the rich presence is also nice
* DONE: It seems like some users have automated updates of their system members. (e.g. every 20-30s).
  * At minimum, we should consider, if the change actually changes anything in our datamodel
    and then only push the change to the other services if it actually causes a change there.
  * furthermore, we should better analyse their usecase and patterns to reduce unnecessary load
  * having a debouce, where updates are not done too frequently are also important.

## Feedback after first deployment of public-test
* member privacy
  * make lists of members / CFs / archived members collapeble and searchable to manage large systems
  * probably start with defaults for actives / archived / CFs and integration with privacy buckets from SP
    * DONE: [privacy buckets in SimplyPlual](https://docs.apparyllis.com/docs/help/features/buckets/intro). Perhaps
      we can also instead make a singleton "pluralsync" account on SP and people can add that one as a friend.
      This way, they can simply assign pluralsync to existing privacy bucket groups and chose what should be shown.
      This is an alternative to asking the users to make a new privacy bucket with the name "pluralsync" which is then read by the API.
  * bidrectional sync of privacy bucket membership and "show in pluralsync" setting
    > If I search for myself, and toggle the "show as fronting" button in SP2A, it autoadds me to the privacy bucket in SP.
    > And if I add myself to the PB in SP, it toggles me as "show as fronting"
  * DONE: add pluralsync user to config explanations and to the pluralsync-deployments as a global singular
  * DONE: testing of privacy features
* DONE: websocket connection restarts
* DONE: better error messages which the users can also understand and which handle most common error paths
  * also let users know, when the VRChat 429 too many requests happen during login - so that they can try again in a day.
* vrchat rate limits hinders pluralsync users to login into VRChat. possibily related to the frequent re-deployments from the same IP-addr on the day before. can we maybe avoid logging in the user at system-startup, then the vrchat cookie already exists from a previous login? what other ways can we use to bypass the rate-limits? maybe do the login in browser instead of via the backend?
* PARTIAL DONE: Add automatic sync to PluralKit
  * DONE: SimplyPlural -> PluralKit sync
  * automatic system sync?
  * set fronter start time based correctly
    * this can be better done, once the plural fetching happens on demand to avoid exessive switches

---

## Backlog

For the next steps, it probably makes sense to announce it in more discord servers and get a larger set of users.
This way we can get even more early testers so that we can then move to the app earlier or later based on the feedback.

* allow back-directional sync of fronters from pluralkit
  * allow automatic import/export of system info between pluralkit and simplyplural
* sync fronting/proxying with plu/ral bot
* long-term: synchronize fronters without requiring users to also use simplyplural. make core independent from simply plural
* checkout inspirations channel in simply plural to see how users use SP and which use cases suit make sense there
* registration logs in by default as well automatically
* security: make it such that on my private instance, only handpicked users may register and use it.
* configureable order in which fronts are shown
* make it more clear, what the people need to do make the discord bridge thing work. maybe a list of steps and if they're working.
* support large systems. i.e. members search and bulk edit.
* **BIG**: APP version so that data and tokens are securely saved in the users local smartphone
  * precondition: make queries.rs into trait and create local implementations for SQLite and Postgres
  * precondition: make HTTP requests layer between frontend and rocket server such that (1) the backend exposes itself as both http endpoints and Tauri commands (via cfg macro) and (2) the front-end uses an interface to decide whether to use Tauri invoke or HTTP requests to access the local/server back-end.
  * This might get complex... Most things should work, but probably not the wwbsocket thing for discord...
  * **alternative: PWA**. see below
* password reset for users
* BUG: when discord rich presence is disabled and the bridge is started, it connects and shows up as "running" though it doesn't show any
  rich presence in discord. this might be confusing. and also, there happens some related errors in the bridge logs which should be investigated
* add status not only for updaters but also for SP itself.
* make sure, that during production, only my own domains are allowed and not localhost or so.
* merge cargo crates into a single workspace to improve build times
* IRRELEVANT?: reduce compile times by removing vrchatapi library and using http rest requests directly

---

## Initial User Feedback before first Prototype
* extend pluralsync to also cover tone-tags / interaction hints as an additional use case? (e.g. IWC = interact-with-care)
