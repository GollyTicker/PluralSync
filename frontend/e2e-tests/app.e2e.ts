import { expect, $, browser } from '@wdio/globals'
import 'mocha'
import { env } from 'process';
import { execSync } from 'child_process';
import axios from 'axios';

const TEST_EMAIL = "test@example.com";
const TEST_PASSWORD = "m?3yp%&wdS+";

const account_test_email = `test-${Date.now()}@example.com`;
const REGISTRATION_PASSWORD = 'a-secure-password';

async function notLoggedIn() {
    await expect($('button[type="submit"]')).toHaveText("Login")
}

async function login(password?: string) {
    await $('#email').setValue(TEST_EMAIL)
    await $('#password').setValue(password ?? TEST_PASSWORD)

    await $('button[type="submit"]').click()
}

async function navigateToLogout() {
    await $('a[href="/logout"]').click();
}

async function loggedInAndOnStatusPage() {
    await expect($('#status-page-title')).toHaveText("Updaters Status")
}

async function navigateToStatus() {
    await $('a[href="/status"]').click();
}

async function loggedInAndOnConfigPage() {
    await expect($('.config-container h1')).toHaveText('Settings');
}

async function navigateToConfig() {
    await $('a[href="/config"]').click();
}

async function configUpdateAndRestartSucceeded() {
    await expect($('#config-update-status')).toHaveText('Config saved successfully and restarted updaters!');
}

// async function configUpdateFailed() {
//     await expect($('#config-update-status')).toHaveText('Failed to save config and restart updaters.');
// }

async function register(email: string) {
    await $('#email').setValue(email);
    await $('#password').setValue(REGISTRATION_PASSWORD);
    await $('button.register-button').click();
}

async function loginWithEmail(email: string, password?: string) {
    await browser.url(env.PLURALSYNC_BASE_URL!);
    await $('#email').setValue(email);
    await $('#password').setValue(password ?? REGISTRATION_PASSWORD);
    await $('button[type="submit"]').click();
}

async function registrationSucceeded() {
    await expect($('.status-message')).toHaveText('Registering your account... A verification link has been sent to your email. Click on it to activate your account!')
}

async function registrationFailed() {
    await expect($('.status-message')).toHaveText('Registration failed: AxiosError: Request failed with status code 409. This email is already being used.');
}

async function navigateToForgotPassword() {
    const forgotPasswordLink = await $('a.forgot-password-link')
    await forgotPasswordLink.click();
}

async function onForgotPasswordPage() {
    await expect($('.forgot-password-container h1')).toHaveText('Forgot Password')
}

async function submitForgotPasswordForm(email: string) {
    await $('#email').setValue(email)
    await $('button[type="submit"]').click()
}

async function forgotPasswordSubmitted() {
    await expect($('.success-message')).toExist()
}

async function onResetPasswordPage() {
    await expect($('.reset-password-container h1')).toHaveText('Reset Password')
}

async function submitResetPasswordForm(newPassword: string) {
    await $('#new-password').setValue(newPassword)
    await $('#confirm-password').setValue(newPassword)
    await $('button[type="submit"]').click()
}

async function resetPasswordSucceeded() {
    await expect($('.success-message')).toExist()
}

async function getResetTokenFromLogs(): Promise<string> {
    try {
        // Source the test script which has the token extraction function
        const token = execSync(
            `bash -c 'cd .. && source test/source.sh && extract_password_reset_token_from_logs "pluralsync-api" >/dev/null 2>&1; echo "$TOKEN"'`,
            { encoding: 'utf8', cwd: process.env.PWD }
        ).trim();

        if (!token) {
            throw new Error('Token extraction returned empty result');
        }

        return token;
    } catch (error) {
        throw new Error(`Failed to extract reset token from logs: ${error}`);
    }
}

async function getVerificationTokenFromLogs(): Promise<string> {
    try {
        // Source the test script which has the token extraction function
        const token = execSync(
            `bash -c 'cd .. && source test/source.sh && extract_verification_token_from_logs "pluralsync-api" >/dev/null 2>&1; echo "$TOKEN"'`,
            { encoding: 'utf8', cwd: process.env.PWD }
        ).trim();

        if (!token) {
            throw new Error('Token extraction returned empty result');
        }

        return token;
    } catch (error) {
        throw new Error(`Failed to extract verification token from logs: ${error}`);
    }
}

async function newAccountVerificationSucceeded() {
    await expect($('.status-message')).toHaveText('Your account has been successfully activated. You can now log in.')
}

async function verifyEmailViaAPI(token: string): Promise<void> {
    try {
        await axios.post(
            `${env.PLURALSYNC_BASE_URL}/api/users/email/verify/${token}`,
            {},
            { headers: { 'Content-Type': 'application/json' } }
        );
    } catch (error) {
        throw new Error(`Failed to verify email: ${error}`);
    }
}


describe('PluralSync registration logic', () => {

    it('should allow a new user to register', async () => {
        await browser.url(env.PLURALSYNC_BASE_URL!);
        await register(account_test_email);
        await registrationSucceeded();
    });

    it('should verify the registered user email', async () => {
        const token = await getVerificationTokenFromLogs();
        await browser.url(env.PLURALSYNC_BASE_URL! + "/verify-email?token=" + token);
        await newAccountVerificationSucceeded();
    });

    it('should allow the new user to log in', async () => {
        await loginWithEmail(account_test_email);
        await loggedInAndOnStatusPage();
    });

    it('should allow the user to log out', async () => {
        await navigateToLogout();
        await notLoggedIn();
    });

    it('should not allow registering with the same email again', async () => {
        await browser.url(env.PLURALSYNC_BASE_URL!);
        await register(account_test_email);
        await registrationFailed();
    });
});

describe('PluralSync password reset logic', () => {
    const newPassword = 'new-secure-password-123!@#';

    it('when user navigates to forgot password page', async () => {
        await browser.url(env.PLURALSYNC_BASE_URL!);
        await navigateToForgotPassword();
        await onForgotPasswordPage();
    });

    it('and user submits forgot password form', async () => {
        await submitForgotPasswordForm(account_test_email);
        await forgotPasswordSubmitted();
    });

    it('and the user resets the password with the token', async () => {
        const token = await getResetTokenFromLogs();
        await browser.url(`${env.PLURALSYNC_BASE_URL}/reset-password?token=${token}`);
        await onResetPasswordPage();
        await submitResetPasswordForm(newPassword);
        await resetPasswordSucceeded();
    });

    it('should not allow login with old password after reset', async () => {
        await browser.url(env.PLURALSYNC_BASE_URL!);
        await loginWithEmail(account_test_email, REGISTRATION_PASSWORD);
        await notLoggedIn();
    });

    it('should allow login with new password', async () => {
        await browser.url(env.PLURALSYNC_BASE_URL!);
        await loginWithEmail(account_test_email, newPassword);
        await loggedInAndOnStatusPage();
    });

    it('should be able to logout after password reset', async () => {
        await navigateToLogout();
        await notLoggedIn();
    });
});

describe('PluralSync login logic', () => {
    it('should be intially not logged in', async () => {
        await browser.url(env.PLURALSYNC_BASE_URL!);
        await notLoggedIn()
    })

    it('can then be logged in to see updater status', async () => {
        await login()
        await loggedInAndOnStatusPage()
    })

    it('can logout and then re-login', async () => {
        await navigateToLogout();
        await notLoggedIn();

        await login();
        await loggedInAndOnStatusPage();
    });

    it('should redirect to login on invalid jwt', async () => {
        await browser.execute(() => {
            window.localStorage.setItem('jwt', '{"inner":"invalid-jwt"}');
        });

        await navigateToConfig();
        await notLoggedIn();
    });
});

describe('PluralSync updater status and config save and restarts', () => {
    it('should show the correct updater status', async () => {
        await browser.url(env.PLURALSYNC_BASE_URL!);
        await login()
        await loggedInAndOnStatusPage()

        await expect($('#VRChat-status')).toHaveText('Disabled');
        await expect($('#ToPluralKit-status')).toHaveText('Running');
        await expect($('#Discord-status')).toHaveText('Starting');
    });

    it('should show the correct config values', async () => {
        await navigateToConfig();
        await loggedInAndOnConfigPage();

        await expect($('#enable_website')).toBeSelected();
        await expect($('#enable_vrchat')).not.toBeSelected();
        await expect($('#enable_discord')).toBeSelected();
        await expect($('#enable_to_pluralkit')).toBeSelected();
        await expect($('#enable_discord_status_message')).toBeSelected();

        await expect($('#website_system_name')).toHaveValue(process.env.WEBSITE_SYSTEM_NAME!);
        await expect($('#website_url_name')).toHaveValue(process.env.WEBSITE_URL_NAME!);

        await expect($('#status_prefix')).toHaveValue("")
        await expect($('#status_no_fronts')).toHaveValue("");
        await expect($('#status_truncate_names_to')).toHaveValue("");

        await expect($('#simply_plural_token')).toHaveValue(process.env.SPS_API_TOKEN!);
        await expect($('#discord_status_message_token')).toHaveValue(process.env.DISCORD_STATUS_MESSAGE_TOKEN!);
    });

    it('should be able to disable discord and pluralkit', async () => {
        await $('#enable_to_pluralkit').click();
        await $('#enable_discord').click();

        await $('button[type="submit"]').click();
        await configUpdateAndRestartSucceeded();

        await navigateToStatus();
        await loggedInAndOnStatusPage();

        await expect($('#VRChat-status')).toHaveText('Disabled');
        await expect($('#ToPluralKit-status')).toHaveText('Disabled');
        await expect($('#Discord-status')).toHaveText('Disabled');
        await expect($('#fronting-status-example')).toHaveText('F: Annalea 💖 A., Borgn B., Daenssa 📶 D., Cstm First');
    });

    it('should be able to re-enable discord and to-pluralkit', async () => {
        await navigateToConfig();
        await loggedInAndOnConfigPage();

        await $('#enable_to_pluralkit').click();
        await $('#enable_discord').click();

        await $('button[type="submit"]').click();
        await configUpdateAndRestartSucceeded();

        await navigateToStatus();
        await loggedInAndOnStatusPage();

        await expect($('#VRChat-status')).toHaveText('Disabled');
        await expect($('#ToPluralKit-status')).toHaveText('Running');
        await expect($('#Discord-status')).toHaveText('Starting');
        await expect($('#fronting-status-example')).toHaveText('F: Annalea 💖 A., Borgn B., Daenssa 📶 D., Cstm First');
    });

    // todo. fix test. when running manually in browser, the field is correctly emptied and an error happens.
    // but the test automation doesn't correctly set the field to empty :/
    // it('should reject invalid configuration', async () => {
    //     await navigateToStatus(); // reset config update status text
    //     await navigateToConfig();
    //     await loggedInAndOnConfigPage();

    //     await expect($('#enable_website')).toBeSelected();
    //     await $('#website_system_name').setValue("");

    //     await expect($('#website_system_name')).toHaveValue("");

    //     await $('button[type="submit"]').click();
    //     await configUpdateFailed();
    // });

    // todo. fix this test
    // it('should correctly save an empty string as an optional value and correctly process numbers', async () => {
    //     await navigateToConfig();
    //     await loggedInAndOnConfigPage();

    //     // Set a value and save it
    //     await $('#wait_seconds').setValue("");
    //     await expect($('#wait_seconds')).toHaveValue("");
    //     await $('button[type="submit"]').click();
    //     await configUpdateAndRestartSucceeded();

    //     // The config is re-fetched on navigation, so the value should be gone
    //     await navigateToStatus();
    //     await navigateToConfig();
    // });
});
