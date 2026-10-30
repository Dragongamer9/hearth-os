// Support Hearth OS: where people can donate, shown in Settings › About › Support Hearth OS and on the website's
// Support section. The owner fills in the links: https addresses only. A link left empty isn't shown, and with none
// filled in the Support page stays hidden everywhere. No payment link is made up here.
// The website has its own copy (site/support-links.js); keep the two the same (hosts/linux/test/test_support_links.py
// checks).
window.HEARTH_SUPPORT = {
  message: 'Hearth OS is free and open source, made by one developer. If it makes your games better, you can help keep it going.',
  links: [
    { id: 'github-sponsors', name: 'GitHub Sponsors', note: 'Monthly or one-time', url: '' },
    { id: 'kofi', name: 'Ko-fi', note: 'Buy the developer a coffee', url: '' },
    { id: 'patreon', name: 'Patreon', note: 'Monthly', url: '' },
    { id: 'paypal', name: 'PayPal', note: 'One-time', url: '' },
  ],
};
