      * Native unit NOCCUR (S2.64): a multiple-occurrence data structure.
      * Subfields in each occurrence, %OCCUR read and set, OCCUR with factor
      * 1 and a result field, an occurrence out of range under MONITOR
      * (00122). The program captured on PUB400, with DSPLY for its output.
      * Source: golden capture tools/golden/native64 n64_occ (PUB400, IBM i
      * 7.5, 2026-11-17).
     H DFTACTGRP(*NO)
     D rec             DS                  OCCURS(3)
     D  code                          3A
     D  qty                           5P 0
     D cur             S              5P 0
     D tot             S              7P 0
     D i               S             10I 0
     D line            S             60A
      /free
        for i = 1 to 3;
          %occur(rec) = i;
          code = 'C' + %char(i);
          qty = i * 100;
        endfor;
        %occur(rec) = 2;
        line = 'OCC2 ' + code + ' ' + %char(qty) + ' AT ' + %char(%occur(rec));
        dsply line;
      /end-free
     C     1             OCCUR     rec           cur
      /free
        line = 'OCC1 ' + code + ' ' + %char(qty) + ' CUR ' + %char(cur);
        dsply line;
        tot = 0;
        for i = 3 downto 1;
          %occur(rec) = i;
          tot += qty;
        endfor;
        line = 'TOTAL ' + %char(tot) + ' AT ' + %char(%occur(rec));
        dsply line;
        i = 4;
        monitor;
          %occur(rec) = i;
        on-error;
          line = 'RANGE ' + %char(%status) + ' AT ' + %char(%occur(rec));
          dsply line;
        endmon;
        *inlr = *on;
      /end-free
