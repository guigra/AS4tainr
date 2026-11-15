**FREE
// Stage 2, S2.41 (ADR 0007): long chains lowered as straight-line code,
// with no recursion: a sum of 200 terms, a concatenation of 150 parts,
// an AND and an OR of 120 conditions, + and * mixed in 100 terms.
// Expected values: integer addition, subtraction and multiplication are
// exact within the result's digits, * before + and -, AND and OR left to
// right (ILE RPG Reference, "Expression Operators", "Precedence Rules",
// "Precision Rules for Numeric Operations", IBM i 7.5,
// https://www.ibm.com/docs/ssw_ibm_i_75/rzasd/expprec.htm, read
// 2026-11-15); 1 + ... + 200 = 20100; the sum of 2i - 1 for i = 1..100
// is 10000. A concatenation is its parts in order.
ctl-opt dftactgrp(*no);
dcl-s n int(10) inz(5);
dcl-s k int(10);
dcl-s s varchar(200);
k = 1 + 2 + 3 + 4 + 5 + 6 + 7 + 8 + 9 + 10 + 11 + 12 + 13 + 14 + 15 + 16 + 17 + 18 + 19 + 20 +
  21 + 22 + 23 + 24 + 25 + 26 + 27 + 28 + 29 + 30 + 31 + 32 + 33 + 34 + 35 + 36 + 37 + 38 +
  39 + 40 + 41 + 42 + 43 + 44 + 45 + 46 + 47 + 48 + 49 + 50 + 51 + 52 + 53 + 54 + 55 + 56 +
  57 + 58 + 59 + 60 + 61 + 62 + 63 + 64 + 65 + 66 + 67 + 68 + 69 + 70 + 71 + 72 + 73 + 74 +
  75 + 76 + 77 + 78 + 79 + 80 + 81 + 82 + 83 + 84 + 85 + 86 + 87 + 88 + 89 + 90 + 91 + 92 +
  93 + 94 + 95 + 96 + 97 + 98 + 99 + 100 + 101 + 102 + 103 + 104 + 105 + 106 + 107 + 108 +
  109 + 110 + 111 + 112 + 113 + 114 + 115 + 116 + 117 + 118 + 119 + 120 + 121 + 122 + 123 +
  124 + 125 + 126 + 127 + 128 + 129 + 130 + 131 + 132 + 133 + 134 + 135 + 136 + 137 + 138 +
  139 + 140 + 141 + 142 + 143 + 144 + 145 + 146 + 147 + 148 + 149 + 150 + 151 + 152 + 153 +
  154 + 155 + 156 + 157 + 158 + 159 + 160 + 161 + 162 + 163 + 164 + 165 + 166 + 167 + 168 +
  169 + 170 + 171 + 172 + 173 + 174 + 175 + 176 + 177 + 178 + 179 + 180 + 181 + 182 + 183 +
  184 + 185 + 186 + 187 + 188 + 189 + 190 + 191 + 192 + 193 + 194 + 195 + 196 + 197 + 198 +
  199 + 200;
dsply ('SUM ' + %char(k));
s = 'A' + 'B' + 'C' + 'D' + 'E' + 'F' + 'G' + 'H' + 'I' + 'J' + 'K' + 'L' + 'M' + 'N' + 'O' +
  'P' + 'Q' + 'R' + 'S' + 'T' + 'U' + 'V' + 'W' + 'X' + 'Y' + 'Z' + 'A' + 'B' + 'C' + 'D' +
  'E' + 'F' + 'G' + 'H' + 'I' + 'J' + 'K' + 'L' + 'M' + 'N' + 'O' + 'P' + 'Q' + 'R' + 'S' +
  'T' + 'U' + 'V' + 'W' + 'X' + 'Y' + 'Z' + 'A' + 'B' + 'C' + 'D' + 'E' + 'F' + 'G' + 'H' +
  'I' + 'J' + 'K' + 'L' + 'M' + 'N' + 'O' + 'P' + 'Q' + 'R' + 'S' + 'T' + 'U' + 'V' + 'W' +
  'X' + 'Y' + 'Z' + 'A' + 'B' + 'C' + 'D' + 'E' + 'F' + 'G' + 'H' + 'I' + 'J' + 'K' + 'L' +
  'M' + 'N' + 'O' + 'P' + 'Q' + 'R' + 'S' + 'T' + 'U' + 'V' + 'W' + 'X' + 'Y' + 'Z' + 'A' +
  'B' + 'C' + 'D' + 'E' + 'F' + 'G' + 'H' + 'I' + 'J' + 'K' + 'L' + 'M' + 'N' + 'O' + 'P' +
  'Q' + 'R' + 'S' + 'T' + 'U' + 'V' + 'W' + 'X' + 'Y' + 'Z' + 'A' + 'B' + 'C' + 'D' + 'E' +
  'F' + 'G' + 'H' + 'I' + 'J' + 'K' + 'L' + 'M' + 'N' + 'O' + 'P' + 'Q' + 'R' + 'S' + 'T';
dsply ('CAT ' + %char(%len(s)) + ' ' + %subst(s:1:30));
if n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and
  n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and
  n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and
  n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and
  n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and
  n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and
  n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and
  n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and
  n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and
  n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and
  n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and
  n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and
  n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and n > 0 and
  n > 0 and n > 0 and n > 0;
  dsply ('AND TRUE');
else;
  dsply ('AND FALSE');
endif;
if n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or
  n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or
  n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or
  n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or
  n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or
  n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or
  n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or
  n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or
  n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or
  n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or
  n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or
  n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n < 0 or n = 5;
  dsply ('OR TRUE');
else;
  dsply ('OR FALSE');
endif;
k = 1 * 2 - 1 + 2 * 2 - 1 + 3 * 2 - 1 + 4 * 2 - 1 + 5 * 2 - 1 + 6 * 2 - 1 + 7 * 2 - 1 + 8 * 2 - 1 +
  9 * 2 - 1 + 10 * 2 - 1 + 11 * 2 - 1 + 12 * 2 - 1 + 13 * 2 - 1 + 14 * 2 - 1 + 15 * 2 - 1 +
  16 * 2 - 1 + 17 * 2 - 1 + 18 * 2 - 1 + 19 * 2 - 1 + 20 * 2 - 1 + 21 * 2 - 1 + 22 * 2 - 1 +
  23 * 2 - 1 + 24 * 2 - 1 + 25 * 2 - 1 + 26 * 2 - 1 + 27 * 2 - 1 + 28 * 2 - 1 + 29 * 2 - 1 +
  30 * 2 - 1 + 31 * 2 - 1 + 32 * 2 - 1 + 33 * 2 - 1 + 34 * 2 - 1 + 35 * 2 - 1 + 36 * 2 - 1 +
  37 * 2 - 1 + 38 * 2 - 1 + 39 * 2 - 1 + 40 * 2 - 1 + 41 * 2 - 1 + 42 * 2 - 1 + 43 * 2 - 1 +
  44 * 2 - 1 + 45 * 2 - 1 + 46 * 2 - 1 + 47 * 2 - 1 + 48 * 2 - 1 + 49 * 2 - 1 + 50 * 2 - 1 +
  51 * 2 - 1 + 52 * 2 - 1 + 53 * 2 - 1 + 54 * 2 - 1 + 55 * 2 - 1 + 56 * 2 - 1 + 57 * 2 - 1 +
  58 * 2 - 1 + 59 * 2 - 1 + 60 * 2 - 1 + 61 * 2 - 1 + 62 * 2 - 1 + 63 * 2 - 1 + 64 * 2 - 1 +
  65 * 2 - 1 + 66 * 2 - 1 + 67 * 2 - 1 + 68 * 2 - 1 + 69 * 2 - 1 + 70 * 2 - 1 + 71 * 2 - 1 +
  72 * 2 - 1 + 73 * 2 - 1 + 74 * 2 - 1 + 75 * 2 - 1 + 76 * 2 - 1 + 77 * 2 - 1 + 78 * 2 - 1 +
  79 * 2 - 1 + 80 * 2 - 1 + 81 * 2 - 1 + 82 * 2 - 1 + 83 * 2 - 1 + 84 * 2 - 1 + 85 * 2 - 1 +
  86 * 2 - 1 + 87 * 2 - 1 + 88 * 2 - 1 + 89 * 2 - 1 + 90 * 2 - 1 + 91 * 2 - 1 + 92 * 2 - 1 +
  93 * 2 - 1 + 94 * 2 - 1 + 95 * 2 - 1 + 96 * 2 - 1 + 97 * 2 - 1 + 98 * 2 - 1 + 99 * 2 - 1 +
  100 * 2 - 1;
dsply ('MIXED ' + %char(k));
*inlr = *on;
