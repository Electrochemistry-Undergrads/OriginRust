// Our own Origin C helper for the Holm / Dunnett reference runs (stats4-*.lines.txt).
// Reads k numeric columns of the active sheet as the groups of a one-way ANOVA, calls Origin's public
// statistics library functions and writes every returned number of the means comparison into columns of
// the same sheet (from column k on) so that the run script can dump the sheet. Nothing here is copied from
// OriginLab code; it only calls documented functions.
#include <Origin.h>
#include <stats_utils.h>

static void put(Worksheet &wks, int col, const vector<double> &v)
{
	while (wks.GetNumCols() <= col)
		wks.AddCol();
	Dataset ds(wks, col);
	ds.SetSize(v.GetSize());
	for (int ip = 0; ip < v.GetSize(); ip++)
		ds[ip] = v[ip];
}

// type: 1 Tukey, 2 Bonferroni, 3 Sidak, 4 Fisher, 5 Scheffe, 6 Dunnett, 7 Bonferroni-Holm, 8 Sidak-Holm
double sheaf_mc(int k, int type, double alpha)
{
	Worksheet wks = Project.ActiveLayer();
	vector<double> data;
	vector<int> gs(k);
	for (int ga = 0; ga < k; ga++)
	{
		Dataset ds(wks, ga);
		for (int ib = 0; ib < ds.GetSize(); ib++)
		{
			if (!is_missing_value(ds[ib]))
			{
				data.Add(ds[ib]);
				gs[ga] = gs[ga] + 1;
			}
		}
	}
	ANOVADescStats desc[12];
	ANOVAStats atot;
	RegANOVA atab;
	int rc = ocmath_anova_one_way(data, data.GetSize(), k, gs, desc, k, &atot, &atab);
	int npair = k * (k - 1) / 2;
	MeanCompStats mc[80];
	int rc2 = stats_anova_mean_comparison(type, desc, &atab, alpha, k, mc, npair);
	vector<double> c0, c1, c2, c3, c4, c5, c6, c7, c8, c9;
	c0.Add(rc);
	c0.Add(rc2);
	for (int i = 0; i < npair; i++)
	{
		c1.Add(mc[i].IndexComp);
		c2.Add(mc[i].MeanDiff);
		c3.Add(mc[i].SEM);
		c4.Add(mc[i].DOF);
		c5.Add(mc[i].Stat);
		c6.Add(mc[i].Prob);
		c7.Add(mc[i].Alpha);
		c8.Add(mc[i].Sig);
		c9.Add(mc[i].LCL);
	}
	vector<double> c10;
	for (int j = 0; j < npair; j++)
		c10.Add(mc[j].UCL);
	put(wks, k, c0);
	put(wks, k + 1, c1);
	put(wks, k + 2, c2);
	put(wks, k + 3, c3);
	put(wks, k + 4, c4);
	put(wks, k + 5, c5);
	put(wks, k + 6, c6);
	put(wks, k + 7, c7);
	put(wks, k + 8, c8);
	put(wks, k + 9, c9);
	put(wks, k + 10, c10);
	return rc + rc2;
}
