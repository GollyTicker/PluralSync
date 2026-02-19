## Steps to first public release version

* move to pluralsync.org for more proper version
* DONE: accounce and get second phase test users by posting about it in more channels
  * DONE: pk server
  * DONE: sp server
  * DONE: rl plural channel
  * pridevr plural channel
    * ticket asking for approval created
  * reddit?
* DONE: improve metrics:
  * measure all the places which "shouldn't happen" in production
    * DONE also do that for important places where errors shouldnt happen.
    * or alternatively, simply setup to regularly look at WARN and ERROR logs?
      and filter theose which are expected away?
  * DONE: distinguish between active and non-active members in metrics
  * DONE: measure length's of suggested fronting statuses
  * measure usage of many specific features
* DONE: Add + test: FireAndForgetChannel with only changes being propagates and another variant with integrated rate-limits
* RESOLVED: complete features from first test users
  * POSTPONED: primarily vrchat rate limit fix
  * REJECTED: possibly also system sync with pluralkit
* add a small log of the latest updates and their timestamps. this log is useful for the users to understand if and when the last update happened
* DONE: extract user agent and make them non-public deployment/build vars
* easily give quick feedback in website - whcih will be saved in db and then I can response to it
* suggested by Aino: make general interviews with a few diverse systems. focus on their needs regardless of pluralsync specifically.
  * getting a hollistic/encompassing understanding is very useful. equally also having a high diversity there.
  * it might also make sense to learn a bit more about plural systems in general and how they use social networks.
    * we can do that in interviews or just browse through reddit and a few places on discord for that
      * on discord just read some channels with plural systems
      * and also checkout the inspirations channel on simply plural discord
  * The core focus is to simply listen and be curious without any specific intention of being a pluralsync user.
* DONE: show example fronters in status page
* DONE: use websocket subscription to simply plural and only get the fronters + system, when it actually changes
  * DONE: and also make the discord websocket thing, that an update is sent immediately once the websocket is created
  * DONE: this also resolves the bottleneck of allowing users to set a 1s update duration
* pluralsync-bridge
  * auto-update
  * DONE: auto-start on system start
* do not clean stuff by default in vrchat. make that configureable
  * add todo to adapt it to work with many other characters as well (chinese, japanese, etc.)
* Remove 'VRChat Status Name' field and change it
  * INSTEAD: change this functionality to have this new name configured in PluralSync UI!
  * and tell users in the setting page, that this is configureable
* make sure, that stuff stays useable in mobile view
* deploy first proper version
* finalize README
* DONE:
  * DONE: add link to Discord Server
  * DONE: add link to KoFi and ask for kind donations
  * DONE: add link to source code
* add data-base backups

## Feedback from second test phase users
* configureable fronting order (details in discord)
* something like this for the website view as well as the fronting link would be good probably: https://codeberg.org/fulmine/pluralkit-xyz
  * their formatting of the rich presence is also nice
* thoughts: It seems like some users have automated updates of their system members. (e.g. every 20-30s).
  * At minimum, we should consider, if the change actually changes anything in our datamodel
    and then only push the change to the other services if it actually causes a change there.
  * furthermore, we should better analyse their usecase and patterns to reduce unnecessary load
  * having a debouce, where updates are not done too frequently are also important.

## Feedback after first deployment of public-test

* member privacy
  * DONE custom fronts configuration
  * DONE archived members configuration / hiding
  * make lists of members / CFs / archived members collapeble and searchable to manage large systems
  * probably start with defaults for actives / archived / CFs and integration with privacy buckets from SP
    * DONE: [privacy buckets in SimplyPlual](https://docs.apparyllis.com/docs/help/features/buckets/intro). Perhaps
      we can also instead make a singleton "pluralsync" account on SP and people can add that one as a friend.
      This way, they can simply assign pluralsync to existing privacy bucket groups and chose what should be shown.
      This is an alternative to asking the users to make a new privacy bucket with the name "pluralsync" which is then read by the API.
    * DONE: privacy bucket API doesn't seem to be documented. I'll have to reverse-engineer that.
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
* DONE: remove `0.1.0` from pluralsync bridge executable
* make sure, that during production, only my own domains are allowed and not localhost or so.
* DONE: restart updaters once in a while, just to get temporary issues out of the way (e.g. vrchat someimes just doesn't work after a re-deployment)
* DONE: make website view such that it doesn't eagery fetch data from simply plural every time but instead uses the latest values from a channel
* merge cargo crates into a single workspace to improve build times
* DONE: better split pluralsync crate into what is exported to bridge-src-tauri and what is not. makes for much faster compiles
* IRRELEVANT?: reduce compile times by removing vrchatapi library and using http rest requests directly
* DONE: complete migration to webapp

* add initial suggestions by ChatAI (e.g. privacy, configurability, etc.)

---

## Initial User Feedback before first Prototype
* DONE: sync to pluralkit as well
* DONE add a warning, that using the discord self-botting comes with a risk for both the user and the dev
  * [artcle by discord](https://support.discord.com/hc/en-us/articles/115002192352-Automated-User-Accounts-Self-Bots)
  * [self-botting](https://gist.github.com/nomsi/2684f5692cad5b0ceb52e308631859fd)
  * [reddit 1](https://old.reddit.com/r/Discord_selfbots/comments/t9o5xf/anyone_got_banned/), [reddit 2](https://old.reddit.com/r/discordapp/comments/7nl35v/regarding_the_ban_on_selfbots/)
  * perhaps use the same approach as used by the discord chat exporter? this might actually work well.
* share with refactionvr server mods before sharing in channel
* extend pluralsync to also cover tone-tags / interaction hints as an additional use case? (e.g. IWC = interact-with-care)

---

## PWA as a Native App Alternative

Summary of the plan to create a PWA instead of native iOS/Android apps to ensure data remains on the user's device.

### Core Architecture
- **Goal**: Avoid app stores (iOS, Android) by using an installable PWA.
- **Rust Code**: Compile the core Rust logic to **WebAssembly (WASM)**.
- **Execution**: Run the WASM module inside a **Web Worker** to handle all data processing on the client-side, preventing UI blocking. This acts as a "local backend".
- **Database**: Since direct SQLite access is not possible in a browser, use **IndexedDB** for storage. To keep SQL-based logic, a WASM-compiled version of SQLite (e.g., `sql.js`, `wa-sqlite`) can be used, which persists its data to IndexedDB.

### Background Tasks & Notifications
The primary challenge is running tasks (e.g., hourly sync, reacting to WebSocket events) when the app is in the background. A PWA cannot maintain a persistent background connection.
The solution is a **server-driven push notification system**.

**Workflow:**
1.  **Server is the Listener/Scheduler**:
    - The backend server (Rocket) listens for external WebSocket events.
    - The backend server runs a cron job for scheduled tasks (e.g., once per hour).
2.  **Server Sends Push Notification**:
    - When an event occurs, the server sends a push notification to the user's device.
3.  **Service Worker Executes Task**:
    - The push notification wakes up the PWA's Service Worker.
    - The Service Worker has a short time window to execute the required task (e.g., fetch data, update IndexedDB). A few seconds is well within the limits.

### Platform-Specific Constraints
- **Requirement**: The user must grant the PWA permission to receive push notifications.
- **Android**: More flexible. Supports "silent" push notifications that can run tasks without displaying a visible alert.
- **iOS**: More restrictive. Background execution is limited to ~30 seconds. To guarantee the task runs, the push notification may need to be user-visible (e.g., display a message or update the app's badge).
