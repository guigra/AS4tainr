**FREE
// Stage 2, S2.152: COMMIT and ROLBK with no commitment control active,
// each in a MONITOR group, the status they leave. Captured on PUB400
// (tools/golden/native152.txt n152_commit) for the native unit NCOMMIT.
ctl-opt dftactgrp(*no);
dcl-s line varchar(80);

monitor;
  commit;
  line = 'commit ok';
on-error;
  line = 'commit ' + %char(%status);
endmon;
monitor;
  rolbk;
  line += ' rolbk ok';
on-error;
  line += ' rolbk ' + %char(%status);
endmon;
say(line);
*inlr = *on;

dcl-proc say;
  dcl-pi *n;
    text varchar(200) const;
  end-pi;
  dcl-s wbuf varchar(200);
  wbuf = text;
  exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out', line => :wbuf,
                                     overwrite => 'APPEND', end_of_line => 'LF');
end-proc;
