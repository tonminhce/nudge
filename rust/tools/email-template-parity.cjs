const main = async () => {
const P = '/app/packages/twenty-emails/dist/index.js';
const M = require(P);
const fs = require('fs');

const out = {};
const jobs = [];
const run = (key, element) => {

  jobs.push(
    Promise.all([
      M.renderEmail(element),
      M.renderEmail(element, { plainText: true }),
    ]).then(([html, text]) => {
      out[key] = { html, text };
    }),
  );
};

run('verification', M.SendEmailVerificationLinkEmail({
  link: 'https://app.twenty.com/verify-email?token=abc123&email=test%40example.com',
  locale: 'en',
  isEmailUpdate: false,
}));
run('verificationUpdate', M.SendEmailVerificationLinkEmail({
  link: 'https://app.twenty.com/verify-email?token=abc123',
  locale: 'en',
  isEmailUpdate: true,
}));
run('passwordReset', M.PasswordResetLinkEmail({
  duration: '24 hours',
  hasPassword: true,
  link: 'https://app.twenty.com/reset-password?token=abc123',
  locale: 'en',
}));
run('passwordResetNoPassword', M.PasswordResetLinkEmail({
  duration: '1 hour',
  hasPassword: false,
  link: 'https://acme.localhost:3000/reset-password?token=xyz',
  locale: 'en',
}));
run('invite', M.SendInviteLinkEmail({
  link: 'https://app.twenty.com/invite/abc',
  workspace: { name: 'Acme Inc.', logo: 'workspaces/7c9e6679/logo.png' },
  sender: { email: 'john.doe@example.com', firstName: 'John', lastName: '' },
  serverUrl: 'https://app.twenty.com',
  locale: 'en',
}));
run('inviteNameOnly', M.SendInviteLinkEmail({
  link: 'https://app.twenty.com/invite/abc',
  workspace: { name: 'Acme Inc.', logo: undefined },
  sender: { email: 'john.doe@example.com', firstName: 'John', lastName: '' },
  serverUrl: 'https://app.twenty.com',
  locale: 'en',
}));
run('inviteLogoOnly', M.SendInviteLinkEmail({
  link: 'https://app.twenty.com/invite/abc',
  workspace: { name: undefined, logo: 'workspaces/7c9e6679/logo.png' },
  sender: { email: 'john.doe@example.com', firstName: 'John', lastName: '' },
  serverUrl: 'https://app.twenty.com',
  locale: 'en',
}));
run('inviteNoLogo', M.SendInviteLinkEmail({
  link: 'https://app.twenty.com/invite/abc',
  workspace: { name: undefined, logo: undefined },
  sender: { email: 'jane@example.com', firstName: 'Jane', lastName: '' },
  serverUrl: 'https://app.twenty.com',
  locale: 'en',
}));
run('passwordUpdate', M.PasswordUpdateNotifyEmail({
  userName: 'John Doe',
  email: 'john.doe@example.com',
  link: 'https://app.twenty.com',
  locale: 'en',
}));
run('twoFactorBackupUsed', M.TwoFactorAuthenticationBackupCodeUsedEmail({
  workspaceDisplayName: 'Acme',
  remainingBackupCodeCount: 9,
  locale: 'en',
}));
run('twoFactorCodesRegen', M.TwoFactorAuthenticationBackupCodesRegeneratedEmail({
  workspaceDisplayName: 'Acme',
  locale: 'en',
}));
run('twoFactorRecoveryIssued', M.TwoFactorAuthenticationRecoveryCodeIssuedEmail({
  actorName: 'John Doe',
  workspaceDisplayName: 'Acme',
  expiresAt: new Date('2026-07-02T10:00:00+00:00'),
  locale: 'en',
}));
run('twoFactorRemoved', M.TwoFactorAuthenticationRemovedEmail({
  workspaceDisplayName: 'Acme',
  locale: 'en',
}));
run('twoFactorReset', M.TwoFactorAuthenticationResetEmail({
  workspaceDisplayName: 'Acme',
  locale: 'en',
}));
run('serverAdmin', M.ServerAdminAccessChangedEmail({
  actorName: 'John Doe',
  targetName: 'Jane Smith',
  targetEmail: 'jane.smith@example.com',
  canAccessFullAdminPanel: true,
  canImpersonate: false,
  locale: 'en',
}));
const AD = M.SendApprovedAccessDomainValidation;

run('approvedDomain', AD({
  link: 'https://app.twenty.com/validate-domain',
  domain: 'example.com',
  workspace: { name: 'Acme Inc.', logo: undefined },
  sender: { email: 'john.doe@example.com', firstName: 'John', lastName: '' },
  serverUrl: 'https://app.twenty.com',
  locale: 'en',
}));
run('approvedDomainFull', AD({
  link: 'https://app.twenty.com/validate-domain',
  domain: 'example.com',
  workspace: { name: 'Acme Inc.', logo: 'workspaces/7c9e6679/logo.png' },
  sender: { email: 'john.doe@example.com', firstName: 'John', lastName: '' },
  serverUrl: 'https://app.twenty.com',
  locale: 'en',
}));
run('approvedDomainLogoOnly', AD({
  link: 'https://app.twenty.com/validate-domain',
  domain: 'example.com',
  workspace: { name: undefined, logo: 'workspaces/7c9e6679/logo.png' },
  sender: { email: 'john.doe@example.com', firstName: 'John', lastName: '' },
  serverUrl: 'https://app.twenty.com',
  locale: 'en',
}));
run('approvedDomainEmpty', AD({
  link: 'https://app.twenty.com/validate-domain',
  domain: 'example.com',
  workspace: { name: undefined, logo: undefined },
  sender: { email: 'john.doe@example.com', firstName: 'John', lastName: '' },
  serverUrl: 'https://app.twenty.com',
  locale: 'en',
}));
run('warnSuspended', M.WarnSuspendedWorkspaceEmail({
  daysSinceInactive: 10,
  inactiveDaysBeforeDelete: 14,
  userName: 'John Doe',
  workspaceDisplayName: 'Acme Inc.',
  link: 'https://acme.twenty.com/settings/billing',
  locale: 'en',
}));
run('cleanSuspended', M.CleanSuspendedWorkspaceEmail({
  daysSinceInactive: 14,
  userName: 'John Doe',
  workspaceDisplayName: 'My Workspace',
  locale: 'en',
}));
run('billingRenewing', M.BillingSubscriptionRenewingEmail({
  userName: 'John Doe',
  workspaceDisplayName: 'Acme Inc.',
  renewsAt: new Date('2026-12-01T00:00:00.000Z'),
  link: 'https://acme.twenty.com/settings/billing',
  locale: 'en',
}));
run('billingRenewingHello', M.BillingSubscriptionRenewingEmail({
  userName: '',
  workspaceDisplayName: 'Acme Inc.',
  renewsAt: new Date('2026-12-01T00:00:00.000Z'),
  link: 'https://acme.twenty.com/settings/billing',
  locale: 'en',
}));
run('billingConvertingMonth', M.BillingTrialConvertingEmail({
  userName: 'John Doe',
  workspaceDisplayName: 'Acme Inc.',
  trialEndsAt: new Date('2026-12-01T00:00:00.000Z'),
  interval: 'month',
  link: 'https://acme.twenty.com/settings/billing',
  locale: 'en',
}));
run('billingConvertingYear', M.BillingTrialConvertingEmail({
  userName: 'John Doe',
  workspaceDisplayName: 'Acme Inc.',
  trialEndsAt: new Date('2026-12-01T00:00:00.000Z'),
  interval: 'year',
  link: 'https://acme.twenty.com/settings/billing',
  locale: 'en',
}));
run('billingEnding', M.BillingTrialEndingEmail({
  userName: 'John Doe',
  workspaceDisplayName: 'Acme Inc.',
  trialEndsAt: new Date('2026-12-01T00:00:00.000Z'),
  dataRetentionDays: 30,
  link: 'https://acme.twenty.com/settings/billing',
  locale: 'en',
}));

await Promise.all(jobs);

const crypto = require('crypto');
const h = (s) => crypto.createHash('sha256').update(s).digest('hex');

for (const [key, { html, text }] of Object.entries(out)) {
  console.log(`${key}\t${h(html)}\t${h(text)}`);
}
};

main();
