{ config, pkgs, ... }: {

  systemd.services.get-device-info = {
    description = "Get device information on startup";
    wantedBy = [ "multi-user.target" ];
    after = [ "sound.target" ];
    User = "root";
    Type = "oneshot";

    serviceConfig = {
      ExecStart = "${pkgs.bash}/bin/bash ./get-device-info.sh > /output/logs/device-info.txt";
    };

  };
}
