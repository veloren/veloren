{ pkgs }: {
  isGitLfsSetup = checkFile:
    let
      gitLfsCheckOutput =
        builtins.readFile (pkgs.runCommand "gitLfsCheck" { } ''
          ([ "$(${pkgs.file}/bin/file --mime-type ${checkFile})" = "${checkFile}: image/png" ] && printf "0" || printf "1") > $out
        '');
    in
    if gitLfsCheckOutput == "0" then
      true
    else
      abort ''
        Git Large File Storage (`git-lfs`) has not been set up correctly.
        Most common reasons:
          - `git-lfs` was not installed before cloning this repository.
          - This repository was not cloned from the primary GitLab mirror.
          - The GitHub mirror does not support LFS.
        See the book at https://book.veloren.net/ for details.
      '';

  # Format number of seconds in the Unix epoch as %Y-%m-%d-%H:%M.
  dateTimeFormat = t:
    let
      rem = x: y: x - x / y * y;
      days = t / 86400;
      secondsInDay = rem t 86400;
      hours = secondsInDay / 3600;
      minutes = (rem secondsInDay 3600) / 60;
      seconds = rem t 60;

      # Courtesy of https://stackoverflow.com/a/32158604.
      z = days + 719468;
      era = (if z >= 0 then z else z - 146096) / 146097;
      doe = z - era * 146097;
      yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
      y = yoe + era * 400;
      doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
      mp = (5 * doy + 2) / 153;
      d = doy - (153 * mp + 2) / 5 + 1;
      m = mp + (if mp < 10 then 3 else -9);
      y' = y + (if m <= 2 then 1 else 0);

      pad = s: if builtins.stringLength s < 2 then "0" + s else s;
    in
    "${toString y'}-${pad (toString m)}-${pad (toString d)}-${pad (toString hours)}:${pad (toString minutes)}";
}
