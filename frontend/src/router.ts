import { createRouter, createWebHistory } from 'vue-router'
import Start from './components/Start.vue'
import Status from './components/Status.vue'
import Config from './components/Config.vue' // Will be renamed to Settings.vue
import Login from './components/Login.vue'
import Logout from './components/Logout.vue'
import ForgotPassword from './components/ForgotPassword.vue'
import ResetPassword from './components/ResetPassword.vue'

const routes: any = [
  { path: '/', component: Start },
  { path: '/login', component: Login },
  { path: '/status', component: Status },
  { path: '/config', component: Config },
  { path: '/logout', component: Logout },
  { path: '/forgot-password', component: ForgotPassword },
  { path: '/reset-password', component: ResetPassword },
]

const router = createRouter({
  history: createWebHistory(),
  routes,
})

export default router
