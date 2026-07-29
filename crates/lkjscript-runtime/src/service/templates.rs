use super::ServiceConfiguration;

pub(super) fn linux_system(configuration: ServiceConfiguration) -> String {
    format!(
        "[Unit]\nDescription=lkjscript machine coordinator\nAfter=local-fs.target\n\n\
         [Service]\nType=simple\nUser={}\nStateDirectory=lkjscript\n\
         ExecStart=/usr/bin/lkjscriptd --foreground --state-dir /var/lib/lkjscript \
         --principal {} --coordinator {}\nRestart=on-failure\nRestartSec=2s\n\
         NoNewPrivileges=true\nPrivateTmp=true\nProtectSystem=strict\n\
         ReadWritePaths=/var/lib/lkjscript\n\n[Install]\nWantedBy=multi-user.target\n",
        configuration.principal, configuration.principal, configuration.coordinator
    )
}

pub(super) fn linux_session() -> String {
    "[Unit]\nDescription=lkjscript interactive session broker\nPartOf=graphical-session.target\n\n\
     [Service]\nType=simple\nExecStart=/usr/bin/lkjscript-session --foreground \
     --endpoint /var/lib/lkjscript/control.sock --backend none\nRestart=on-failure\n\
     NoNewPrivileges=true\n\n[Install]\nWantedBy=graphical-session.target\n"
        .to_string()
}

pub(super) fn windows(configuration: ServiceConfiguration) -> String {
    format!(
        "# Run in an elevated installer; the service never presents UI.\n\
         sc.exe create lkjscriptd start= auto binPath= \
         \"\\\"C:\\Program Files\\lkjscript\\lkjscriptd.exe\\\" --service \
         --state-dir C:\\ProgramData\\lkjscript --principal {} --coordinator {}\"\n\
         # A per-login lkjscript-session broker is registered separately.\n",
        configuration.principal, configuration.coordinator
    )
}

pub(super) fn macos_daemon(configuration: ServiceConfiguration) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \
         \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <plist version=\"1.0\"><dict>\n<key>Label</key><string>org.lkjscript.daemon</string>\n\
         <key>ProgramArguments</key><array>\n<string>/usr/local/libexec/lkjscriptd</string>\n\
         <string>--foreground</string><string>--state-dir</string>\n\
         <string>/Library/Application Support/lkjscript</string>\n\
         <string>--principal</string><string>{}</string>\n\
         <string>--coordinator</string><string>{}</string>\n</array>\n\
         <key>RunAtLoad</key><true/><key>KeepAlive</key><true/>\n</dict></plist>\n",
        configuration.principal, configuration.coordinator
    )
}

pub(super) fn macos_agent() -> String {
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
     <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \
     \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
     <plist version=\"1.0\"><dict>\n<key>Label</key><string>org.lkjscript.session</string>\n\
     <key>ProgramArguments</key><array>\n<string>/usr/local/libexec/lkjscript-session</string>\n\
     <string>--foreground</string><string>--endpoint</string>\n\
     <string>/Library/Application Support/lkjscript/control.sock</string>\n\
     <string>--backend</string><string>none</string>\n</array>\n<key>RunAtLoad</key><true/>\n</dict></plist>\n"
        .to_string()
}

pub(super) fn container(configuration: ServiceConfiguration) -> String {
    format!(
        "exec /usr/bin/lkjscriptd --foreground --state-dir /var/lib/lkjscript \
         --principal {} --coordinator {}\n",
        configuration.principal, configuration.coordinator
    )
}
