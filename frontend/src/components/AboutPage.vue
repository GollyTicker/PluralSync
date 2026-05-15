<script setup lang="ts">
import { screenshots } from '../screenshots'

export interface Screenshot {
  /** Filename of the screenshot */
  filename: string
  /** Public URL path to the screenshot */
  path: string
  /** Name derived from filename (without extension) */
  name: string
}
</script>

<template>
  <div class="about-page">
    <h1>About PluralSync</h1>

    <div class="content-section">
      <p>
        PluralSync is a cloud service where users can automatically sync their plural systems'
        fronting status between various system managers and social platforms such as
        <a href="https://pluralkit.me/" target="_blank">PluralKit</a>,
        <a href="https://apparyllis.com/" target="_blank">SimplyPlural</a>,
        <a href="https://hello.vrchat.com/" target="_blank">VRChat</a>,
        <a href="https://discord.com" target="_blank">Discord</a> or their own website. Users of
        system managers (plural systems, DID/OSDD systems, etc.) benefit from this as it makes it
        easier for them to communicate who's fronting while only needing to update their fronting on
        Simply Plural.
      </p>

      <ul>
        <li>
          <a href="https://discord.gg/nHmNKxbv8v" target="_blank">
            Community & Feedback Discord Server
          </a>
        </li>
        <li><router-link to="/announcements">Announcements</router-link></li>
        <li>
          <a
            href="https://radicle.network/nodes/radicle.ayake.net/rad%3AzxqCqSgaUsXM5a5Jd2fVBktEJi2J"
            target="_blank"
            >Source Code on Radicle (rad:zxqCqSgaUsXM5a5Jd2fVBktEJi2J)</a
          >
        </li>
        <li><a href="https://ko-fi.com/ayake_ayake#" target="_blank">Support Us 💖</a></li>
      </ul>

      <h2>Developer(s)</h2>
      <p>
        Developed with ❤️ by <a href="https://github.com/GollyTicker/" target="_blank">Ayake</a>.
        <br />
        <a href="https://ko-fi.com/ayake_ayake#" target="_blank">Support Us 💖</a>
      </p>

      <h2>Supported Synchronizations</h2>
      <ul>
        <li>
          SimplyPlural to VRChat Status, Discord Status, Discord Rich Presence / Activity, Website
          with fronter names and avatars
        </li>
        <li>SimplyPlural to PluralKit Fronters Switch</li>
      </ul>
      <ul>
        <li>
          PluralKit to VRChat Status, Discord Status, Discord Rich Presence / Activity, Website with
          fronter names and avatars
        </li>
      </ul>

      <h2>Open Source, Free & Libre Software</h2>
      <p>
        The source code is <strong>libre/free</strong> software. You can find it on
        <a
          href="https://radicle.network/nodes/radicle.ayake.net/rad%3AzxqCqSgaUsXM5a5Jd2fVBktEJi2J"
          target="_blank"
          >Radicle (rad:zxqCqSgaUsXM5a5Jd2fVBktEJi2J)</a
        >
        <br />
        You have the freedom to run, copy, distribute, study, change and improve the software under
        the Copyleft
        <a
          href="https://www.tldrlegal.com/license/gnu-affero-general-public-license-v3-agpl-3-0"
          target="_blank"
        >
          AGPL
        </a>
        license.
        <br />
        <a href="https://www.gnu.org/philosophy/free-sw.en.html" target="_blank"
          >What is libre/free software?</a
        >
        <br />
        You miss something? Ask for support in the community server!
        <br />
        We don't implement something you want? Copy the code, modify it and run it yourself!
        <br />
        You have different values than us? Great! Use this project as an inspiration to solve your
        problems with your set of values and make your project public so that others can use them.
        We like this kind of "competition" and diversity of solutions.
      </p>

      <h2>Security & Privacy</h2>
      <p>
        We take data security and privacy seriously. The data to synchronise between the services is
        stored encrypted and at industry-standard security. Self hosting is possible if you have
        some tech knowledge.
      </p>

      <h2>Pragmatically Sustainable AI-assisted Coding</h2>
      <p>
        This software was written with AI-assistance to enhance development and make things possible
        which would be not possible otherwise. The code is reviewed equally to hand-written code to
        ensure good quality, safe and reliable software. We're professional software engineers, so
        we're not dependent on the AI for quality. This is AI-assisted software engineering and not
        "AI-slop". The AI is used as a "Pair Programmer" to assist coding. We use this because it
        helps us a lot and because we can provide features to the users.
        <br />
        The AI coding assistant only has access to the code (which is public anyways).
        <strong> The AI does not have access to user data on the server </strong>
        - as there is a strict isolation between them.
        <br />
        We run the LLM for the coding assistant locally and offline - which is furthermore private,
        under our control and is financially and energetically sustainable. We don't spend on
        companies providing AI/LLMs to not encourage their data-stealing practises.
        <br />
        We strive to use AI pragmatically and beneficially. Our actions align with our
        interpretation of reasonable AI usage - and we prefer to use AI rather than not use it.
      </p>

      <h2>Background Image</h2>
      <p>
        The photo
        <a href="https://www.nasa.gov/image-detail/fd02_for-pao/"
          >Hello, World by NASA/Reid Wiseman</a
        >
        was taken on 2nd April 2026 on the Artemis II mission around the moon.<br />
        ❤️ In the harsh times, this picture unites us all and symbolises how we've all lived on the
        same planet for our entire lives and humanity. ❤️
      </p>
    </div>

    <div class="screenshots-section">
      <h2>Screenshots</h2>
      <div class="screenshot-grid">
        <div v-for="screenshot in screenshots" :key="screenshot.filename" class="screenshot-item">
          <img :src="screenshot.path" />
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.about-page {
  max-width: 800px;
  margin: 0 auto;
  padding: 2rem 1rem;
}

h1 {
  margin-bottom: 1.5rem;
  text-align: center;
}

h2 {
  margin-top: 2rem;
  margin-bottom: 1rem;
  font-size: 1.4em;
}

.content-section {
  background-color: #f8f9fa;
  border: 1px solid #dee2e6;
  border-radius: 0.5em;
  padding: 1.5rem;
  margin-bottom: 2rem;
}

.content-section p {
  line-height: 1.6;
  color: #333;
  margin-bottom: 1rem;
}

.content-section ul {
  line-height: 1.8;
  color: #333;
  margin: 1rem 0;
  padding-left: 1.5rem;
}

.content-section a {
  color: var(--color-secondary);
  text-decoration: none;
}

.content-section a:hover {
  text-decoration: underline;
}

.screenshots-section {
  background-color: #f8f9fa;
  border: 1px solid #dee2e6;
  border-radius: 0.5em;
  padding: 1.5rem;
  margin-bottom: 2rem;
}

.screenshot-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
  gap: 1.5rem;
  margin-top: 1rem;
}

.screenshot-item {
  background-color: white;
  border: 1px solid #e9ecef;
  border-radius: 0.25em;
  overflow: hidden;
}

.screenshot-item img {
  width: 100%;
  height: auto;
  display: block;
}
</style>
