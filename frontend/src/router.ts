import type { RouteRecordRaw } from 'vue-router'
import { createRouter, createWebHistory } from 'vue-router'
import StartPage from './components/StartPage.vue'
import StatusDisplay from './components/StatusDisplay.vue'
import ConfigSettings from './components/ConfigSettings.vue'
import DeleteAccount from './components/DeleteAccount.vue'
import LoginPage from './components/LoginPage.vue'
import LogoutButton from './components/LogoutButton.vue'
import ForgotPassword from './components/ForgotPassword.vue'
import ResetPassword from './components/ResetPassword.vue'
import VerifyEmail from './components/VerifyEmail.vue'
import HistoryTab from './components/HistoryTab.vue'

const routes: RouteRecordRaw[] = [
  { path: '/', component: StartPage },
  { path: '/login', component: LoginPage },
  { path: '/status', component: StatusDisplay },
  { path: '/history', component: HistoryTab },
  { path: '/config', component: ConfigSettings },
  { path: '/settings/delete-account', component: DeleteAccount },
  { path: '/logout', component: LogoutButton },
  { path: '/forgot-password', component: ForgotPassword },
  { path: '/reset-password', component: ResetPassword },
  { path: '/verify-email', component: VerifyEmail },
]

const router = createRouter({
  history: createWebHistory(),
  routes,
})

export default router
