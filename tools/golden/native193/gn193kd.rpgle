**FREE
// Stage 2, S2.193: CHAIN with %KDS into a LIKEREC data structure: both
// keys, the first key only (%KDS(ds : 1)), and a key not found; the
// program fields stay as they were. Captured on PUB400
// (tools/golden/native193.txt n193_kds) for the native unit NKDS.
ctl-opt dftactgrp(*no);
dcl-f GN193PF keyed;
dcl-ds rec likerec(GN193FMT);
dcl-ds k qualified;
  o zoned(3 : 0);
  l packed(3 : 0);
end-ds;
dcl-s line varchar(200);

k.o = 1;
k.l = 3;
chain %kds(k) GN193PF rec;
line = %trim(rec.name) + ' ' + %char(%found(GN193PF));
k.o = 2;
k.l = 7;
chain %kds(k : 1) GN193PF rec;
line += ' ' + %trim(rec.name) + ' ' + %char(rec.lin) + ' ' + %char(%found(GN193PF));
k.o = 9;
chain %kds(k) GN193PF rec;
line += ' ' + %trim(rec.name) + ' ' + %char(%found(GN193PF));
line += ' [' + %trim(name) + ']';
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
