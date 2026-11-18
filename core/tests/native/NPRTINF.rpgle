**FREE
// Native unit NPRTINF (review of S2.79, 2026-11-18): a printer file with
// an INFDS, the program captured on PUB400 as GN92INF with DSPLY for its
// output; NPRTINFP is the capture's GN92P (the same DDS as NPRTWP).
// Source: golden capture tools/golden/native92 n92_infds (PUB400, IBM i
// 7.5, 2026-11-18).
ctl-opt dftactgrp(*no);
dcl-f nprtinfp printer infds(inf);
dcl-ds inf;
  f_status *status;
  f_opcode *opcode;
  f_record *record;
  f_line int(5) pos(367);
  f_page int(10) pos(369);
end-ds;
amt = 1234.5;
nm = 'ALPHA';
qty = 7;
write hdr;
show();
write dtl;
show();
close nprtinfp;
show();
*inlr = *on;

dcl-proc show;
  dsply (%trim(f_opcode) + '|' + %trim(f_record) + '|' + %char(f_status)
      + '|' + %char(f_line) + '|' + %char(f_page));
end-proc;

