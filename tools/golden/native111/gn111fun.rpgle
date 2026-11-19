**FREE
// The review of 2026-11-19 (ADR 0008): a decimal part under unary minus
// in a float expression keeps its precision rule: with p packed(30:0)
// and q packed(2:2) = .99, p + q has no room for decimals. Also + of a
// float. Captured on PUB400 (tools/golden/native111.txt n111_floatunary)
// for the native unit NFLTUNA.
ctl-opt dftactgrp(*no);
dcl-s f float(8);
dcl-s res float(8);
dcl-s p packed(30:0);
dcl-s q packed(2:2) inz(.99);
dcl-s line varchar(200);
res = -(f + (p + q));
line = %char(res);
f = 2;
res = +(f * (q + 1));
line += ' ' + %char(res);
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
