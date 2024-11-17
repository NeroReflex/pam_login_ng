# login-ng
A greeter that shields the real password behind another password that can be unlocked by various means:
    - __autologin__: provide autologin functionality that has been long lost in systemd-homed
    - __secondary password(s)__: allow the use of one or more secondary passwords
    - __controllers__: enter a password via a gaming controller
    - __fingerprint__: no password required: login via fingerprint
    - __files__ use a specific file on some kind of removable media to authenticate
    - __pin__ a numeric pin just as in your phone

By default login-ng will behave exactly as any other greeter: you type your password to access your account.

Using *login-ng_ctl* utility you can set more authentication options (or even configure autologin) as well
as providing a custom command to execute upon a successful login.
