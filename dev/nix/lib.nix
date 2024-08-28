{lib, ...}: {
  categorize = let
    category = set:
      map (package:
        if lib.hasAttr "command" package
        then package // {inherit (set) category;}
        else {
          inherit package;
          inherit (set) category;
        })
      set.pkgs;
  in
    l: lib.flatten (map category l);
}
